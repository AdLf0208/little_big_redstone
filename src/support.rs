//! Game glue for the microchip: the block entity that holds the circuit, the per-face redstone
//! answers, the level-tick evaluation and the `/lbr` configuration surface.
//!
//! Upstream ticks the microchip's block entity every server tick (`MicrochipBlockEntity.tick`),
//! evaluates its components in traversal order and writes the six face properties plus targeted
//! neighbour updates when a signal changed. A write inside a block entity's own tick does not
//! survive at this pin, so the work is split: `block_entity_tick` only registers the position for
//! the tick, and `level_tick_post` reads the circuit, evaluates it and writes the compound, the
//! face properties and the neighbour notifications where the level is in place.
//!
//! Two facts of the SDK shape the glue:
//!
//! * `block_signal` is asked from inside the level's update and may not read the world, and a
//!   `&dyn LevelHost` cannot hand out a block-entity handle, so the per-face power a microchip
//!   emits is kept in this module's registry (filled from the saved compound in
//!   `block_entity_tick`, and from the evaluation's result) instead of being read back from the
//!   block entity as the retired bundle did.
//! * `LevelRef::signal` reads what a block gives its neighbour in one direction, so an input port
//!   now sees the game's own `getSignal` answer — another mod's `block_signal` included. The
//!   retired bundle rebuilt that answer from the neighbour's public state.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Mutex, MutexGuard, PoisonError};

use mod_api::{
    BlockEntityEvent, BlockEntityPart, BlockEntityTickEvent, BlockEntityTickReply,
    BlockEntityTicksEvent, BlockPart, BlockSignalEvent, Command, CommandArgument,
    CommandArgumentValue, CommandInvocation, Compound, Direction, Flow, Host, IsSignalSourceEvent,
    LevelHost, LevelRef, LevelSide, NeighbourUpdate, Registrar, RunModCommandEvent, ServerHost,
    Tag, Text, WorldLifecycleEvent,
};

use crate::runtime::{
    AndConfig, Circuit, Command as Action, CommandError, Comparison, Component, ComponentKind,
    FACE_COUNT, Face, IoConfig, Live, Port, PowerType, Wire,
};

/// The one block this slice carries.
const BLOCK: &str = "little_big_redstone:red_microchip";
/// Its block entity type, in this mod's own namespace.
const BLOCK_ENTITY_TYPE: &str = "red_microchip";
/// The compound holding the circuit, as upstream's `MicrochipBlockEntity` does.
const CIRCUIT_TAG: &str = "microchip";
/// The last emitted power per face (upstream keeps it in its redstone awareness; the port caches
/// it in the saved compound so `block_signal` is a read, not an evaluation).
const POWER_TAG: &str = "power";
/// Whether each face's output port is strong (upstream's `outputStrongSides`).
const STRONG_TAG: &str = "strong";
/// The circuit format's version, so a later slice can migrate.
const FORMAT_VERSION: i32 = 1;
/// The command roots the port answers; `runtime::parse_command` owns the same pair.
const ROOTS: [&str; 2] = ["lbr", "little_big_redstone"];

/// What one microchip emits, by [`Face::index`]: the power and whether the output is strong.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Power {
    power: [u8; FACE_COUNT],
    strong: [bool; FACE_COUNT],
}

/// This tick's registered microchips, and the last power each one emits.
#[derive(Default)]
struct Registry {
    /// The level whose block entities registered; a different one starts a new table.
    level: Option<LevelRef>,
    /// Positions to evaluate at the end of this tick.
    chips: BTreeSet<[i32; 3]>,
    /// The last answer of every microchip, by position, for `block_signal`.
    power: BTreeMap<[i32; 3], Power>,
}

static STATE: Mutex<Option<Registry>> = Mutex::new(None);

fn registry() -> MutexGuard<'static, Option<Registry>> {
    STATE.lock().unwrap_or_else(PoisonError::into_inner)
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

/// Declare the microchip's block entity type and attach the Part that answers for it.
pub fn declare(r: &mut dyn Registrar) {
    let block = r.block_key(BLOCK);
    let kind = r
        .block_entity_type(BLOCK_ENTITY_TYPE)
        .blocks(&["red_microchip"])
        .build();
    r.block_part(block, Microchip);
    r.block_entity_part(&kind, Microchip)
        .expect("the microchip's Part attaches to the type this mod just declared");
}

/// The microchip's Part: the block's redstone answers and its block entity's tick.
pub struct Microchip;

#[mod_api::horizon_part]
impl BlockPart for Microchip {
    /// The stored power on the face the signal leaves through, weak or direct
    /// (`MicrochipBlock.getSignal` / `getDirectSignal`).
    fn block_signal(&self, _level: &dyn LevelHost, event: &mut BlockSignalEvent) {
        let index = to_face(event.face).index();
        let power = cached_power(at(event.pos));
        event.signal = if event.direct {
            if power.strong[index] {
                i32::from(power.power[index])
            } else {
                0
            }
        } else {
            i32::from(power.power[index])
        };
    }

    /// A microchip answers on its output faces, so redstone wire connects to it
    /// (`MicrochipBlock.isSignalSource`).
    fn is_signal_source(&self, _level: &dyn LevelHost, event: &mut IsSignalSourceEvent) {
        event.source = true;
    }
}

#[mod_api::horizon_part]
impl BlockEntityPart for Microchip {
    /// A fresh microchip starts with an empty compound, as the retired bundle's
    /// `native_block_entity` did.
    fn block_entity(
        &self,
        _level: LevelSide,
        _event: &mut BlockEntityEvent<'_>,
    ) -> Option<Compound> {
        Some(Compound::new())
    }

    /// The microchip ticks on the server.
    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    /// One server tick: register the position for this tick's evaluation and keep
    /// `block_signal`'s answer in step with the saved compound. Nothing is written here — a block
    /// entity's own tick cannot persist its compound.
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        let power = stored_power(&event.entity.nbt);
        let Some(level) = level_in_place(host) else {
            return;
        };
        let pos = event.pos;
        {
            let mut guard = registry();
            let registry = guard.get_or_insert_with(Registry::default);
            if registry.level != Some(level) {
                *registry = Registry {
                    level: Some(level),
                    ..Registry::default()
                };
            }
            registry.chips.insert(pos);
            registry.power.insert(pos, power);
        }
        *event.reply = BlockEntityTickReply::Unchanged;
    }
}

// ---------------------------------------------------------------------------
// The level tick
// ---------------------------------------------------------------------------

/// The level tick is over: evaluate every microchip registered this tick.
pub fn level_tick_post(host: &mut dyn ServerHost, level: LevelRef) {
    if !level.in_place(host) {
        return;
    }
    let positions: Vec<[i32; 3]> = {
        let mut guard = registry();
        let Some(registry) = guard.as_mut() else {
            return;
        };
        if registry.level != Some(level) {
            return;
        }
        std::mem::take(&mut registry.chips).into_iter().collect()
    };
    for pos in positions {
        evaluate(host, level, pos);
    }
}

/// Evaluate one microchip: read its circuit and the signal on each face, evaluate, then persist
/// the outputs and sync the block's face properties.
fn evaluate(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> Option<Live> {
    if !is_microchip_at(host, level, pos) {
        return None;
    }
    let before = read_circuit(host, level, pos);
    if before.is_empty() {
        return None;
    }
    let power_before = before.power();
    let mut circuit = before.clone();
    let live = circuit.evaluate(face_signals(host, level, pos));
    let power_after = circuit.power();
    let compound = circuit_compound(&circuit, &live);
    remember_power(pos, power_of_compound(&compound));
    if circuit != before {
        write_microchip(host, level, pos, compound);
    }
    sync_faces(host, level, pos, &live, power_before != power_after);
    Some(live)
}

/// Write the six face properties when they changed, and tell the neighbours when a face's emitted
/// power changed (`RedstoneAwareness.postTick`).
fn sync_faces(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    live: &Live,
    power_changed: bool,
) {
    let mut changed = false;
    for face in Face::ALL {
        let want = if live.active[face.index()] {
            "true"
        } else {
            "false"
        };
        if level.block_property(host, pos, face.name()).as_deref() == Some(want) {
            continue;
        }
        changed |= host.set_block_property(pos, face.name(), want);
    }
    if !(changed || power_changed) {
        return;
    }
    if let Some(source) = level.block_state(host, pos) {
        let _ = level.update_neighbors(host, pos, source, NeighbourUpdate::Neighbours);
    }
}

/// The signal arriving at each face from the block beside it.
fn face_signals(host: &dyn Host, level: LevelRef, pos: [i32; 3]) -> [u8; FACE_COUNT] {
    let mut inputs = [0u8; FACE_COUNT];
    for face in Face::ALL {
        inputs[face.index()] = signal_toward(host, level, pos, face);
    }
    inputs
}

/// `getSignal(neighbour, face)`: what the block on `face` gives toward the microchip, by the
/// game's own read (`LevelRef::signal`), this port's `block_signal` included.
fn signal_toward(host: &dyn Host, level: LevelRef, pos: [i32; 3], face: Face) -> u8 {
    let neighbour = offset(pos, face);
    let signal = level
        .signal(host, neighbour, direction(face))
        .unwrap_or_default();
    u8::try_from(signal.clamp(0, 15)).unwrap_or_default()
}

/// A block position as the plain array the port's runtime speaks.
fn at(pos: mod_api::BlockPos) -> [i32; 3] {
    [pos.x, pos.y, pos.z]
}

/// The level whose tick this is: a block entity's tick is not told its level.
fn level_in_place(host: &mut dyn ServerHost) -> Option<LevelRef> {
    host.levels()
        .list_levels()
        .into_iter()
        .find(|level| level.in_place(host))
}

// ---------------------------------------------------------------------------
// Saved state
// ---------------------------------------------------------------------------

/// The saved `microchip` compound, or an empty circuit when there is none or it is unreadable.
fn read_circuit(host: &dyn Host, level: LevelRef, pos: [i32; 3]) -> Circuit {
    host.block_entity(level, pos)
        .and_then(|entity| entity.data(host))
        .and_then(|data| match field(&data, CIRCUIT_TAG) {
            Some(Tag::Compound(compound)) => Some(circuit_from_nbt(compound)),
            _ => None,
        })
        .unwrap_or_default()
}

fn write_microchip(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3], compound: Compound) {
    let mut data = Compound::new();
    data.insert(CIRCUIT_TAG.into(), Tag::Compound(compound));
    if let Some(entity) = host.block_entity(level, pos) {
        let _ = entity.merge_data(host, data);
    }
}

/// The per-face power and strength flags, zero when there is no circuit.
fn stored_power(nbt: &Compound) -> Power {
    match field(nbt, CIRCUIT_TAG) {
        Some(Tag::Compound(compound)) => power_of_compound(compound),
        _ => Power::default(),
    }
}

fn power_of_compound(compound: &Compound) -> Power {
    let mut power = Power::default();
    if let Some(Tag::IntArray(values)) = field(compound, POWER_TAG) {
        for (slot, value) in power.power.iter_mut().zip(values) {
            *slot = u8::try_from((*value).clamp(0, 15)).unwrap_or_default();
        }
    }
    if let Some(Tag::ByteArray(values)) = field(compound, STRONG_TAG) {
        for (slot, value) in power.strong.iter_mut().zip(values) {
            *slot = *value != 0;
        }
    }
    power
}

fn cached_power(pos: [i32; 3]) -> Power {
    registry()
        .as_ref()
        .and_then(|registry| registry.power.get(&pos).copied())
        .unwrap_or_default()
}

fn remember_power(pos: [i32; 3], power: Power) {
    let mut guard = registry();
    guard
        .get_or_insert_with(Registry::default)
        .power
        .insert(pos, power);
}

fn is_microchip_at(host: &dyn Host, level: LevelRef, pos: [i32; 3]) -> bool {
    let Some(block) = host.block_key(BLOCK) else {
        return false;
    };
    level.block(host, pos) == Some(block)
}

// ---------------------------------------------------------------------------
// The command surface
// ---------------------------------------------------------------------------

/// Declare the `/lbr` usages this slice implements (`register_commands`, API 8).
pub fn declare_commands(commands: &mut Vec<Command>) {
    for root in ROOTS {
        commands.push(Command::new(&format!("{root} help")));
        commands.push(Command::new(&format!(
            "{root} circuit <pos:block_pos> status"
        )));
        commands.push(Command::new(&format!(
            "{root} circuit <pos:block_pos> clear"
        )));
        commands.push(Command::new(&format!(
            "{root} circuit <pos:block_pos> and <input_a:word> <input_b:word> <output:word>"
        )));
    }
}

/// Run a `/lbr` line (`run_mod_command`, API 8, which replaces `perform_command`). Upstream edits a
/// circuit in the microchip's GUI; the port offers the same operations on the command line (a
/// documented deviation).
pub fn run_command(host: &mut dyn ServerHost, event: &mut RunModCommandEvent<'_>) -> Flow {
    let invocation: &CommandInvocation = event.command;
    match action(&invocation.literals, &invocation.arguments) {
        Ok(Action::Unknown) => Flow::Continue,
        Ok(command) => {
            perform(host, command, invocation.dimension);
            Flow::Cancel
        }
        Err(error) => {
            say(host, format!("lbr: {error} (try /lbr help)"));
            Flow::Cancel
        }
    }
}

/// The typed invocation as this port's own action. The dispatcher has already parsed and resolved
/// the arguments, so this is the retired parser's job done on typed values; the refusals are the
/// runtime's own [`CommandError`], so the replies are the ones 0.1.0 gave.
fn action(literals: &[String], arguments: &[CommandArgument]) -> Result<Action, CommandError> {
    let root = literals.first().map_or("", String::as_str);
    if !ROOTS.contains(&root) {
        return Ok(Action::Unknown);
    }
    match literals.get(1).map(String::as_str) {
        Some("help") => Ok(Action::Help),
        Some("circuit") => {
            let Some(pos) = block_pos(argument(arguments, "pos")) else {
                return Err(CommandError::WrongArity);
            };
            match literals.get(2).map(String::as_str) {
                Some("status") => Ok(Action::Status { pos }),
                Some("clear") => Ok(Action::Clear { pos }),
                Some("and") => {
                    let inputs = [
                        face_argument(arguments, "input_a")?,
                        face_argument(arguments, "input_b")?,
                    ];
                    let output = face_argument(arguments, "output")?;
                    Ok(Action::Configure {
                        pos,
                        inputs,
                        output,
                    })
                }
                Some(other) => Err(CommandError::UnknownAction(other.to_owned())),
                None => Err(CommandError::WrongArity),
            }
        }
        Some(other) => Err(CommandError::UnknownAction((*other).to_owned())),
        None => Err(CommandError::WrongArity),
    }
}

fn argument<'a>(arguments: &'a [CommandArgument], name: &str) -> Option<&'a CommandArgumentValue> {
    arguments
        .iter()
        .rev()
        .find(|argument| argument.name == name)
        .map(|argument| &argument.value)
}

fn block_pos(value: Option<&CommandArgumentValue>) -> Option<[i32; 3]> {
    match value {
        Some(CommandArgumentValue::BlockPos(pos)) => Some(*pos),
        _ => None,
    }
}

fn face_argument(arguments: &[CommandArgument], name: &str) -> Result<Face, CommandError> {
    let Some(CommandArgumentValue::String(token)) = argument(arguments, name) else {
        return Err(CommandError::WrongArity);
    };
    Face::parse(token).ok_or_else(|| CommandError::UnknownFace(token.clone()))
}

/// Run one action and answer through the command's feedback.
fn perform(host: &mut dyn ServerHost, action: Action, dimension: mod_api::DimensionKey) {
    let Some(level) = dimension.id(host).and_then(|name| host.level(&name)) else {
        say(host, "lbr: no such dimension".to_owned());
        return;
    };
    match action {
        Action::Unknown => {}
        Action::Help => {
            for line in [
                "/lbr circuit <x> <y> <z> and <input_a> <input_b> <output>",
                "  Build an AND gate with two input ports and one output port.",
                "/lbr circuit <x> <y> <z> status",
                "  Report the microchip's circuit and its last live signals.",
                "/lbr circuit <x> <y> <z> clear",
                "  Empty the microchip.",
            ] {
                say(host, line.to_owned());
            }
        }
        Action::Configure {
            pos,
            inputs,
            output,
        } => {
            if !is_microchip_at(host, level, pos) {
                say(
                    host,
                    format!("lbr: no microchip at {} {} {}", pos[0], pos[1], pos[2]),
                );
                return;
            }
            let circuit = match Circuit::with_and_gate(inputs[0], inputs[1], output) {
                Ok(circuit) => circuit,
                Err(error) => {
                    say(host, format!("lbr: {error}"));
                    return;
                }
            };
            let live = write_circuit(host, level, pos, &circuit);
            say(
                host,
                format!(
                    "lbr: AND gate at {} {} {}: inputs {} and {}, output {}",
                    pos[0],
                    pos[1],
                    pos[2],
                    inputs[0].name(),
                    inputs[1].name(),
                    output.name()
                ),
            );
            report_live(host, pos, &live);
        }
        Action::Status { pos } => {
            if !is_microchip_at(host, level, pos) {
                say(
                    host,
                    format!("lbr: no microchip at {} {} {}", pos[0], pos[1], pos[2]),
                );
                return;
            }
            let circuit = read_circuit(host, level, pos);
            if circuit.is_empty() {
                say(
                    host,
                    format!(
                        "lbr: the microchip at {} {} {} has no circuit",
                        pos[0], pos[1], pos[2]
                    ),
                );
                return;
            }
            for line in describe(&circuit) {
                say(host, line);
            }
            let live = evaluate(host, level, pos).unwrap_or_else(Live::dark);
            report_live(host, pos, &live);
        }
        Action::Clear { pos } => {
            if !is_microchip_at(host, level, pos) {
                say(
                    host,
                    format!("lbr: no microchip at {} {} {}", pos[0], pos[1], pos[2]),
                );
                return;
            }
            clear_circuit(host, level, pos);
            say(
                host,
                format!(
                    "lbr: cleared the microchip at {} {} {}",
                    pos[0], pos[1], pos[2]
                ),
            );
        }
    }
}

/// The `/lbr ... and` command: store the circuit, evaluate it once and return what the faces see.
fn write_circuit(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    circuit: &Circuit,
) -> Live {
    let mut circuit = circuit.clone();
    let live = circuit.evaluate(face_signals(host, level, pos));
    let compound = circuit_compound(&circuit, &live);
    remember_power(pos, power_of_compound(&compound));
    write_microchip(host, level, pos, compound);
    sync_faces(host, level, pos, &live, true);
    live
}

fn clear_circuit(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3]) {
    if let Some(entity) = host.block_entity(level, pos) {
        let _ = entity.replace_data(host, Compound::new());
    }
    remember_power(pos, Power::default());
    sync_faces(host, level, pos, &Live::dark(), true);
}

fn say(host: &mut dyn ServerHost, line: String) {
    let _ = host.commands().feedback(Text::literal(line));
}

fn report_live(host: &mut dyn ServerHost, pos: [i32; 3], live: &Live) {
    let mut parts = Vec::new();
    for face in Face::ALL {
        if live.active[face.index()] || live.power[face.index()] > 0 {
            parts.push(format!(
                "{}: in {} out {}",
                face.name(),
                live.inputs[face.index()],
                live.power[face.index()]
            ));
        }
    }
    if parts.is_empty() {
        parts.push("all faces dark".to_owned());
    }
    say(
        host,
        format!(
            "lbr: microchip at {} {} {}: {}",
            pos[0],
            pos[1],
            pos[2],
            parts.join(", ")
        ),
    );
}

/// The circuit's components and wires, one line each, for `/lbr ... status`.
fn describe(circuit: &Circuit) -> Vec<String> {
    let mut lines = Vec::new();
    for component in circuit.components() {
        lines.push(match component.kind {
            ComponentKind::Io(config) => format!(
                "lbr: slot {}: I/O {} on {} (strength {}, {}, {}), output {}",
                component.slot,
                if config.input { "input" } else { "output" },
                config.face.name(),
                config.signal_strength,
                comparison_name(config.comparison),
                power_name(config.power_type),
                component.output
            ),
            ComponentKind::And(config) => format!(
                "lbr: slot {}: AND gate with {} inputs, output {}",
                component.slot, config.inputs, component.output
            ),
        });
    }
    for wire in circuit.wires() {
        lines.push(format!(
            "lbr: wire {}#{} -> {}#{}",
            wire.output.slot, wire.output.index, wire.input.slot, wire.input.index
        ));
    }
    lines
}

fn comparison_name(comparison: Comparison) -> &'static str {
    match comparison {
        Comparison::LessThanOrEqual => "at most",
        Comparison::Equal => "exactly",
        Comparison::GreaterThanOrEqual => "at least",
    }
}

fn power_name(power: PowerType) -> &'static str {
    match power {
        PowerType::Weak => "weak",
        PowerType::Strong => "strong",
    }
}

fn comparison_code(comparison: Comparison) -> &'static str {
    match comparison {
        Comparison::LessThanOrEqual => "less_than_or_equal_to",
        Comparison::Equal => "equal_to",
        Comparison::GreaterThanOrEqual => "greater_than_or_equal_to",
    }
}

fn comparison_from(text: &str) -> Comparison {
    match text {
        "less_than_or_equal_to" => Comparison::LessThanOrEqual,
        "equal_to" => Comparison::Equal,
        _ => Comparison::GreaterThanOrEqual,
    }
}

fn power_code(power: PowerType) -> &'static str {
    match power {
        PowerType::Weak => "weak",
        PowerType::Strong => "strong",
    }
}

fn power_from(text: &str) -> PowerType {
    match text {
        "strong" => PowerType::Strong,
        _ => PowerType::Weak,
    }
}

// ---------------------------------------------------------------------------
// Directions
// ---------------------------------------------------------------------------

fn direction(face: Face) -> Direction {
    match face {
        Face::Down => Direction::Down,
        Face::Up => Direction::Up,
        Face::North => Direction::North,
        Face::South => Direction::South,
        Face::West => Direction::West,
        Face::East => Direction::East,
    }
}

fn to_face(direction: Direction) -> Face {
    match direction {
        Direction::Down => Face::Down,
        Direction::Up => Face::Up,
        Direction::North => Face::North,
        Direction::South => Face::South,
        Direction::West => Face::West,
        Direction::East => Face::East,
    }
}

fn offset([x, y, z]: [i32; 3], face: Face) -> [i32; 3] {
    match face {
        Face::Down => [x, y - 1, z],
        Face::Up => [x, y + 1, z],
        Face::North => [x, y, z - 1],
        Face::South => [x, y, z + 1],
        Face::West => [x - 1, y, z],
        Face::East => [x + 1, y, z],
    }
}

// ---------------------------------------------------------------------------
// The circuit's saved compound
// ---------------------------------------------------------------------------

/// The tag saved under `name`, by the plain name the game's own readers use.
fn field<'a>(nbt: &'a Compound, name: &str) -> Option<&'a Tag> {
    nbt.iter()
        .find(|(key, _)| key.to_string_lossy() == name)
        .map(|(_, tag)| tag)
}

/// The circuit as the saved compound: components, wires and the per-face power cache. Field names
/// follow upstream's codec where the slice has the concept (`components`, `wires`, `config`,
/// `output`, `slot`, `index`).
fn circuit_compound(circuit: &Circuit, live: &Live) -> Compound {
    let mut components = Vec::with_capacity(circuit.components().len());
    for component in circuit.components() {
        let mut entry = Compound::new();
        entry.insert("slot".into(), Tag::Int(component.slot));
        entry.insert("output".into(), Tag::Int(i32::from(component.output)));
        let mut config = Compound::new();
        match component.kind {
            ComponentKind::Io(io) => {
                entry.insert("type".into(), Tag::String("io".into()));
                config.insert("input".into(), Tag::Byte(i8::from(io.input)));
                config.insert("direction".into(), Tag::String(io.face.name().into()));
                config.insert(
                    "signal_strength".into(),
                    Tag::Int(i32::from(io.signal_strength)),
                );
                config.insert(
                    "signal_comparison".into(),
                    Tag::String(comparison_code(io.comparison).into()),
                );
                config.insert(
                    "power_type".into(),
                    Tag::String(power_code(io.power_type).into()),
                );
            }
            ComponentKind::And(and) => {
                entry.insert("type".into(), Tag::String("and_gate".into()));
                config.insert("input_count".into(), Tag::Int(i32::from(and.inputs)));
            }
        }
        entry.insert("config".into(), Tag::Compound(config));
        components.push(Tag::Compound(entry));
    }
    let mut wires = Vec::with_capacity(circuit.wires().len());
    for wire in circuit.wires() {
        let mut entry = Compound::new();
        entry.insert("output".into(), Tag::Compound(port_to_nbt(wire.output)));
        entry.insert("input".into(), Tag::Compound(port_to_nbt(wire.input)));
        wires.push(Tag::Compound(entry));
    }
    let power: Vec<i32> = live.power.iter().map(|value| i32::from(*value)).collect();
    let strong: Vec<u8> = circuit
        .components()
        .iter()
        .filter_map(|component| match component.kind {
            ComponentKind::Io(io) if !io.input => Some(io),
            _ => None,
        })
        .fold(vec![0u8; FACE_COUNT], |mut strong, io| {
            let index = io.face.index();
            if strong[index] == 0 {
                strong[index] = u8::from(io.power_type == PowerType::Strong);
            }
            strong
        });
    let mut compound = Compound::new();
    compound.insert("version".into(), Tag::Int(FORMAT_VERSION));
    compound.insert("components".into(), Tag::List(components));
    compound.insert("wires".into(), Tag::List(wires));
    compound.insert("power".into(), Tag::IntArray(power));
    compound.insert("strong".into(), Tag::ByteArray(strong));
    compound
}

fn port_to_nbt(port: Port) -> Compound {
    let mut compound = Compound::new();
    compound.insert("slot".into(), Tag::Int(port.slot));
    compound.insert("index".into(), Tag::Int(i32::from(port.index)));
    compound
}

/// Read a circuit, skipping entries this slice cannot express. A tampered file gives an empty
/// board, never a panic.
fn circuit_from_nbt(compound: &Compound) -> Circuit {
    let mut circuit = Circuit::new();
    if let Some(Tag::List(entries)) = field(compound, "components") {
        for entry in entries {
            let Tag::Compound(entry) = entry else {
                continue;
            };
            if let Some(component) = component_from_nbt(entry) {
                circuit.push_component(component);
            }
        }
    }
    if let Some(Tag::List(entries)) = field(compound, "wires") {
        for entry in entries {
            let Tag::Compound(entry) = entry else {
                continue;
            };
            let (Some(output), Some(input)) = (
                field(entry, "output").and_then(port_from_nbt),
                field(entry, "input").and_then(port_from_nbt),
            ) else {
                continue;
            };
            let known = |slot: i32| {
                circuit
                    .components()
                    .iter()
                    .any(|component| component.slot == slot)
            };
            if known(output.slot) && known(input.slot) {
                circuit.push_wire(Wire { output, input });
            }
        }
    }
    circuit
}

fn component_from_nbt(entry: &Compound) -> Option<Component> {
    let slot = match field(entry, "slot")? {
        Tag::Int(slot) => *slot,
        _ => return None,
    };
    let output = match field(entry, "output") {
        Some(Tag::Int(output)) => u8::try_from((*output).clamp(0, 15)).unwrap_or_default(),
        _ => 0,
    };
    let kind = match field(entry, "type")? {
        Tag::String(kind) if kind.to_string_lossy() == "io" => {
            let Some(Tag::Compound(config)) = field(entry, "config") else {
                return None;
            };
            let face = match field(config, "direction") {
                Some(Tag::String(face)) => Face::parse(face.to_string_lossy().as_str())?,
                _ => Face::North,
            };
            let input = matches!(field(config, "input"), Some(Tag::Byte(value)) if *value != 0);
            let signal_strength = match field(config, "signal_strength") {
                Some(Tag::Int(strength)) => u8::try_from((*strength).clamp(0, 15)).unwrap_or(1),
                _ => 1,
            };
            let comparison = match field(config, "signal_comparison") {
                Some(Tag::String(comparison)) => {
                    comparison_from(comparison.to_string_lossy().as_str())
                }
                _ => Comparison::GreaterThanOrEqual,
            };
            let power_type = match field(config, "power_type") {
                Some(Tag::String(power)) => power_from(power.to_string_lossy().as_str()),
                _ => PowerType::Weak,
            };
            ComponentKind::Io(IoConfig {
                input,
                face,
                signal_strength,
                comparison,
                power_type,
            })
        }
        Tag::String(kind) if kind.to_string_lossy() == "and_gate" => {
            let inputs = match field(entry, "config") {
                Some(Tag::Compound(config)) => match field(config, "input_count") {
                    Some(Tag::Int(inputs)) => u8::try_from((*inputs).clamp(0, 255)).unwrap_or(2),
                    _ => AndConfig::MIN_INPUTS,
                },
                _ => AndConfig::MIN_INPUTS,
            };
            ComponentKind::And(AndConfig::new(inputs))
        }
        _ => return None,
    };
    Some(Component { slot, kind, output })
}

fn port_from_nbt(tag: &Tag) -> Option<Port> {
    let Tag::Compound(compound) = tag else {
        return None;
    };
    let slot = match field(compound, "slot")? {
        Tag::Int(slot) => *slot,
        _ => return None,
    };
    let index = match field(compound, "index")? {
        Tag::Int(index) => u8::try_from((*index).clamp(0, 255)).unwrap_or_default(),
        _ => return None,
    };
    Some(Port { slot, index })
}

// ---------------------------------------------------------------------------
// World lifecycle
// ---------------------------------------------------------------------------

/// A world opened or closed: the registered positions belong to the world that is going away
/// (`morered`'s `world_lifecycle` finding).
pub fn world_lifecycle(_event: &mut WorldLifecycleEvent<'_>) {
    *registry() = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn io_circuit() -> Circuit {
        Circuit::with_and_gate(Face::North, Face::South, Face::East).expect("distinct faces")
    }

    fn live_with(power: [u8; FACE_COUNT]) -> Live {
        let mut live = Live::dark();
        live.power = power;
        live
    }

    #[test]
    fn a_circuit_round_trips_through_the_saved_compound() {
        let mut circuit = io_circuit();
        let live = circuit.evaluate({
            let mut inputs = [0u8; FACE_COUNT];
            inputs[Face::North.index()] = 15;
            inputs[Face::South.index()] = 5;
            inputs
        });
        let compound = circuit_compound(&circuit, &live);
        let read = circuit_from_nbt(&compound);
        assert_eq!(read, circuit, "components and wires survive a save");
        assert_eq!(
            read.power(),
            circuit.power(),
            "the power cache is rebuilt from the components"
        );
    }

    #[test]
    fn the_saved_power_and_strength_arrays_are_read_back() {
        let mut circuit = io_circuit();
        let live = circuit.evaluate({
            let mut inputs = [0u8; FACE_COUNT];
            inputs[Face::North.index()] = 15;
            inputs[Face::South.index()] = 15;
            inputs
        });
        let compound = circuit_compound(&circuit, &live);
        assert_eq!(
            field(&compound, POWER_TAG),
            Some(&Tag::IntArray(vec![0, 0, 0, 0, 0, 15])),
            "east (index 5) emits 15, every other face is dark"
        );
        assert_eq!(
            field(&compound, STRONG_TAG),
            Some(&Tag::ByteArray(vec![0, 0, 0, 0, 0, 0])),
            "the slice's output port is weak, as upstream's default"
        );
    }

    #[test]
    fn a_tampered_compound_loads_as_an_empty_circuit() {
        let mut compound = Compound::new();
        let mut unknown = Compound::new();
        unknown.insert("slot".into(), Tag::Int(0));
        unknown.insert("type".into(), Tag::String("randomizer".into()));
        compound.insert(
            "components".into(),
            Tag::List(vec![
                Tag::Compound(unknown),
                Tag::String("not a component".into()),
            ]),
        );
        let mut port = Compound::new();
        port.insert("slot".into(), Tag::Int(0));
        port.insert("index".into(), Tag::Int(0));
        let mut wire = Compound::new();
        wire.insert("output".into(), Tag::Compound(port));
        compound.insert("wires".into(), Tag::List(vec![Tag::Compound(wire)]));
        let circuit = circuit_from_nbt(&compound);
        assert!(
            circuit.is_empty(),
            "unknown components and dangling wires are dropped"
        );
    }

    #[test]
    fn a_missing_circuit_reads_as_empty() {
        assert!(circuit_from_nbt(&Compound::new()).is_empty());
    }

    #[test]
    fn every_comparison_and_power_mode_round_trips() {
        for comparison in [
            Comparison::LessThanOrEqual,
            Comparison::Equal,
            Comparison::GreaterThanOrEqual,
        ] {
            assert_eq!(comparison_from(comparison_code(comparison)), comparison);
        }
        for power in [PowerType::Weak, PowerType::Strong] {
            assert_eq!(power_from(power_code(power)), power);
        }
    }

    #[test]
    fn the_power_cache_survives_a_strong_output_port() {
        let mut circuit = Circuit::new();
        circuit.push_component(Component {
            slot: 0,
            kind: ComponentKind::Io(IoConfig {
                input: false,
                face: Face::East,
                signal_strength: 15,
                power_type: PowerType::Strong,
                ..IoConfig::default()
            }),
            output: 15,
        });
        let live = live_with([0, 0, 0, 0, 0, 15]);
        let compound = circuit_compound(&circuit, &live);
        assert_eq!(
            field(&compound, STRONG_TAG),
            Some(&Tag::ByteArray(vec![0, 0, 0, 0, 0, 1])),
            "a strong output port marks its face"
        );
    }

    #[test]
    fn every_face_maps_to_its_vanilla_direction_and_back() {
        for face in Face::ALL {
            assert_eq!(to_face(direction(face)), face);
            assert_eq!(direction(face) as u8, face.index() as u8);
        }
    }

    /// The typed invocation the dispatcher hands `run_mod_command` for
    /// `/lbr circuit 0 64 0 and north south east`.
    #[test]
    fn the_typed_arguments_become_the_slice_circuit() {
        assert_eq!(
            action(
                &words(&["lbr", "circuit", "and"]),
                &[
                    argument("pos", CommandArgumentValue::BlockPos([0, 64, 0])),
                    argument("input_a", CommandArgumentValue::String("north".to_owned())),
                    argument("input_b", CommandArgumentValue::String("south".to_owned())),
                    argument("output", CommandArgumentValue::String("east".to_owned())),
                ],
            ),
            Ok(Action::Configure {
                pos: [0, 64, 0],
                inputs: [Face::North, Face::South],
                output: Face::East,
            })
        );
    }

    #[test]
    fn a_foreign_command_is_not_this_ports() {
        assert_eq!(
            action(&words(&["enderstorage", "info"]), &[]),
            Ok(Action::Unknown)
        );
    }

    #[test]
    fn the_typed_arguments_refuse_what_the_line_parser_refused() {
        assert_eq!(
            action(
                &words(&["lbr", "circuit", "and"]),
                &[
                    argument("pos", CommandArgumentValue::BlockPos([0, 64, 0])),
                    argument("input_a", CommandArgumentValue::String("north".to_owned())),
                    argument(
                        "input_b",
                        CommandArgumentValue::String("sideways".to_owned())
                    ),
                    argument("output", CommandArgumentValue::String("east".to_owned())),
                ],
            ),
            Err(CommandError::UnknownFace("sideways".to_owned()))
        );
        assert_eq!(
            action(
                &words(&["lbr", "circuit", "flip"]),
                &[argument("pos", CommandArgumentValue::BlockPos([0, 64, 0]))],
            ),
            Err(CommandError::UnknownAction("flip".to_owned()))
        );
        assert_eq!(
            action(&words(&["lbr", "circuit", "status"]), &[]),
            Err(CommandError::WrongArity)
        );
        assert_eq!(action(&words(&["lbr", "help"]), &[]), Ok(Action::Help));
    }

    fn words(items: &[&str]) -> Vec<String> {
        items.iter().map(|text| (*text).to_owned()).collect()
    }

    fn argument(name: &str, value: CommandArgumentValue) -> CommandArgument {
        CommandArgument::new(name.to_owned(), value)
    }

    #[test]
    fn a_compound_key_is_read_by_its_plain_name() {
        let mut compound = Compound::new();
        compound.insert(CIRCUIT_TAG.into(), Tag::Int(7));
        assert_eq!(field(&compound, CIRCUIT_TAG), Some(&Tag::Int(7)));
        assert_eq!(field(&compound, "nothing"), None);
    }
}
