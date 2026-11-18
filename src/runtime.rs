// Little Big Redstone — the pure microchip model.
//
// Ported from **Little Big Redstone 1.9.13-1.21.1** (`Swedz/Little-Big-Redstone`,
// MIT). This file is the standalone crate library (`cargo test` runs it without
// the game) and is also injected into the game as `crate::game::little_big_redstone`
// by `mixins/little_big_redstone/horizon-mixin.toml`.
//
// Everything here is a rewrite of the *behaviour* read from the mod's public
// guide pages shipped in its jar (`assets/little_big_redstone/guides/...`) and
// from its MIT source at commit `f08bd39c` — never a translation of its code.
// The numbers and rules are recorded in `FINDINGS.md`.
//
// No game types: faces, ports, components, wires, the analog gate table and the
// command surface are plain Rust.

use std::fmt;

/// How many faces a microchip has, matching vanilla `Direction`'s ordinals.
pub const FACE_COUNT: usize = 6;

/// A microchip's face, in vanilla `Direction` order (down, up, north, south,
/// west, east) so a face index is the world direction's ordinal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Face {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl Face {
    /// Every face, in ordinal order.
    pub const ALL: [Face; FACE_COUNT] = [
        Face::Down,
        Face::Up,
        Face::North,
        Face::South,
        Face::West,
        Face::East,
    ];

    /// The face's vanilla ordinal, usable as an index into the per-face arrays.
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// The face on the other side of the block.
    #[must_use]
    pub const fn opposite(self) -> Face {
        match self {
            Face::Down => Face::Up,
            Face::Up => Face::Down,
            Face::North => Face::South,
            Face::South => Face::North,
            Face::West => Face::East,
            Face::East => Face::West,
        }
    }

    /// The lower-case name the guide and the command use (`logic/io.md`).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Face::Down => "down",
            Face::Up => "up",
            Face::North => "north",
            Face::South => "south",
            Face::West => "west",
            Face::East => "east",
        }
    }

    /// Parse a face name; case-sensitive, as the guide spells them.
    #[must_use]
    pub fn parse(text: &str) -> Option<Face> {
        Face::ALL.into_iter().find(|face| face.name() == text)
    }
}

/// `LogicComparisonMode`: how an input port compares the signal it receives with
/// its configured strength (`logic/io.md`, `LogicComparisonMode.java`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comparison {
    LessThanOrEqual,
    Equal,
    GreaterThanOrEqual,
}

impl Comparison {
    /// `LogicComparisonMode.test`.
    #[must_use]
    pub const fn test(self, input: u8, setting: u8) -> bool {
        match self {
            Comparison::LessThanOrEqual => input <= setting,
            Comparison::Equal => input == setting,
            Comparison::GreaterThanOrEqual => input >= setting,
        }
    }
}

/// `LogicPowerOutputType`: whether an output port weakly or strongly powers the
/// block on its face.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerType {
    Weak,
    Strong,
}

/// An I/O port's configuration (`LogicIOConfig`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IoConfig {
    /// Input mode reads the face; output mode drives it.
    pub input: bool,
    /// The microchip face this port is on.
    pub face: Face,
    /// The signal strength the port compares against (input) or emits (output).
    pub signal_strength: u8,
    /// How an input compares the incoming signal.
    pub comparison: Comparison,
    /// How an output powers its face.
    pub power_type: PowerType,
}

impl Default for IoConfig {
    /// `LogicIOConfig.DEFAULT`: input, north, strength 1, "at least", weak.
    fn default() -> Self {
        Self {
            input: true,
            face: Face::North,
            signal_strength: 1,
            comparison: Comparison::GreaterThanOrEqual,
            power_type: PowerType::Weak,
        }
    }
}

impl IoConfig {
    /// The number of input ports this port has.
    #[must_use]
    pub const fn input_ports(self) -> u8 {
        if self.input { 0 } else { 1 }
    }
}

/// A multi-input gate's configuration (`MultiLogicGateConfig`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AndConfig {
    /// How many inputs the gate requires, `2..=10`.
    pub inputs: u8,
}

impl AndConfig {
    /// The fewest inputs an AND gate takes (`inputPortsAllowed()`).
    pub const MIN_INPUTS: u8 = 2;
    /// The most inputs an AND gate takes (`inputPortsAllowed()`).
    pub const MAX_INPUTS: u8 = 10;

    /// A configuration clamped into `MIN_INPUTS..=MAX_INPUTS`, as upstream's
    /// constructor does.
    #[must_use]
    pub const fn new(inputs: u8) -> Self {
        Self {
            inputs: if inputs < Self::MIN_INPUTS {
                Self::MIN_INPUTS
            } else if inputs > Self::MAX_INPUTS {
                Self::MAX_INPUTS
            } else {
                inputs
            },
        }
    }
}

impl Default for AndConfig {
    fn default() -> Self {
        Self {
            inputs: Self::MIN_INPUTS,
        }
    }
}

/// What a component is. The slice implements the two the guide's AND scene
/// uses; the rest of the component library is a recorded gap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentKind {
    Io(IoConfig),
    And(AndConfig),
}

impl ComponentKind {
    /// `LogicComponent.inputPorts`.
    #[must_use]
    pub const fn input_ports(self) -> u8 {
        match self {
            ComponentKind::Io(config) => config.input_ports(),
            ComponentKind::And(config) => config.inputs,
        }
    }
}

/// One component on the board, with the output it last produced. Upstream stores
/// that output in the component's NBT (`LogicGate.mapCodec`'s `output` field), so
/// it is part of the saved state, not a scratch value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Component {
    /// The board slot the component occupies (`LogicEntry.slot`).
    pub slot: i32,
    /// What the component is.
    pub kind: ComponentKind,
    /// The signal it last produced on each of its output ports (0 when off).
    pub output: u8,
}

/// One end of a wire: a component slot and one of its ports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Port {
    /// The component's board slot.
    pub slot: i32,
    /// The port's index on that component.
    pub index: u8,
}

/// A wire from an output port to an input port (`Wire`). Wires carry a signal
/// with no loss, as the guide's scenes assume.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Wire {
    /// The port the signal comes from.
    pub output: Port,
    /// The port it goes to.
    pub input: Port,
}

/// What one evaluation saw and produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Live {
    /// The signal arriving at each face, by [`Face::index`].
    pub inputs: [u8; FACE_COUNT],
    /// The signal each face emits, by [`Face::index`].
    pub power: [u8; FACE_COUNT],
    /// Whether each face is live (an input seeing power, or an output emitting),
    /// which is the block's six face properties upstream.
    pub active: [bool; FACE_COUNT],
}

impl Live {
    /// Nothing arriving, nothing emitted.
    #[must_use]
    pub const fn dark() -> Self {
        Self {
            inputs: [0; FACE_COUNT],
            power: [0; FACE_COUNT],
            active: [false; FACE_COUNT],
        }
    }
}

impl Default for Live {
    fn default() -> Self {
        Self::dark()
    }
}

/// A microchip's circuit: components and the wires between them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Circuit {
    components: Vec<Component>,
    wires: Vec<Wire>,
}

impl Circuit {
    /// An empty board.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The guide's own AND scene (`logic/and_gate.md`): two input ports, one
    /// two-input AND gate and one output port, wired `A → 0`, `B → 1`,
    /// `gate → out`.
    ///
    /// # Errors
    /// [`CommandError::DuplicateFace`] when two of the three faces coincide — a
    /// face may carry one port only (`logic/io.md`).
    pub fn with_and_gate(input_a: Face, input_b: Face, output: Face) -> Result<Self, CommandError> {
        if input_a == input_b {
            return Err(CommandError::DuplicateFace(input_a));
        }
        if output == input_a || output == input_b {
            return Err(CommandError::DuplicateFace(output));
        }
        let mut circuit = Self::new();
        for (slot, face) in [(0, input_a), (1, input_b)] {
            circuit.push_component(Component {
                slot,
                kind: ComponentKind::Io(IoConfig {
                    face,
                    ..IoConfig::default()
                }),
                output: 0,
            });
        }
        circuit.push_component(Component {
            slot: 2,
            kind: ComponentKind::And(AndConfig::default()),
            output: 0,
        });
        circuit.push_component(Component {
            slot: 3,
            kind: ComponentKind::Io(IoConfig {
                input: false,
                face: output,
                signal_strength: 15,
                ..IoConfig::default()
            }),
            output: 0,
        });
        for (source, target) in [(0, 0), (1, 1)] {
            circuit.push_wire(Wire {
                output: Port {
                    slot: source,
                    index: 0,
                },
                input: Port {
                    slot: 2,
                    index: target,
                },
            });
        }
        circuit.push_wire(Wire {
            output: Port { slot: 2, index: 0 },
            input: Port { slot: 3, index: 0 },
        });
        Ok(circuit)
    }

    /// The board's components, in storage order.
    #[must_use]
    pub fn components(&self) -> &[Component] {
        &self.components
    }

    /// The board's wires, in storage order.
    #[must_use]
    pub fn wires(&self) -> &[Wire] {
        &self.wires
    }

    /// Whether the board holds nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }

    /// Add a component (a loader builds a board this way).
    pub fn push_component(&mut self, component: Component) {
        self.components.push(component);
    }

    /// Add a wire (a loader builds a board this way).
    pub fn push_wire(&mut self, wire: Wire) {
        self.wires.push(wire);
    }

    /// The order the components tick in: every component after the ones feeding
    /// it, ties broken by slot. Components in a feedback loop come last, in slot
    /// order (upstream's BFS traversal orders a cycle differently; the slice
    /// builds acyclic circuits — recorded as a deviation).
    #[must_use]
    pub fn order(&self) -> Vec<usize> {
        let mut indegree = vec![0usize; self.components.len()];
        let mut edges: Vec<(usize, usize)> = Vec::new();
        for wire in &self.wires {
            let (Some(source), Some(target)) = (
                self.index_of(wire.output.slot),
                self.index_of(wire.input.slot),
            ) else {
                continue;
            };
            if source == target || edges.contains(&(source, target)) {
                continue;
            }
            edges.push((source, target));
            indegree[target] += 1;
        }
        let mut order: Vec<usize> = Vec::with_capacity(self.components.len());
        let mut ready: Vec<usize> = (0..self.components.len())
            .filter(|index| indegree[*index] == 0)
            .collect();
        while !ready.is_empty() {
            ready.sort_by_key(|index| self.components[*index].slot);
            let index = ready.remove(0);
            order.push(index);
            for (source, target) in &edges {
                if *source == index {
                    indegree[*target] -= 1;
                    if indegree[*target] == 0 {
                        ready.push(*target);
                    }
                }
            }
        }
        if order.len() < self.components.len() {
            let mut rest: Vec<usize> = (0..self.components.len())
                .filter(|index| !order.contains(index))
                .collect();
            rest.sort_by_key(|index| self.components[*index].slot);
            order.extend(rest);
        }
        order
    }

    /// The faces carrying both an input and an output port. Both ports are inert
    /// there (`logic/io.md`: "neither port will work").
    #[must_use]
    pub fn io_conflicts(&self) -> Vec<Face> {
        Face::ALL
            .into_iter()
            .filter(|face| {
                let mut input = false;
                let mut output = false;
                for component in &self.components {
                    if let ComponentKind::Io(config) = component.kind
                        && config.face == *face
                    {
                        if config.input {
                            input = true;
                        } else {
                            output = true;
                        }
                    }
                }
                input && output
            })
            .collect()
    }

    /// Evaluate one tick: every component reads its wired inputs in dependency
    /// order and produces its output; then each face's emitted power and
    /// liveness is derived. `inputs` is the signal arriving at each face from the
    /// block beside it.
    pub fn evaluate(&mut self, inputs: [u8; FACE_COUNT]) -> Live {
        for index in self.order() {
            let component = self.components[index];
            let wired = self.port_inputs(component);
            let next = match component.kind {
                ComponentKind::Io(config) => {
                    if self.io_conflicts().contains(&config.face) {
                        // An invalid pair does not process (`processTick` checks
                        // `isConfigValid`); it keeps its last output.
                        component.output
                    } else if config.input {
                        io_input(&config, inputs[config.face.index()])
                    } else {
                        wired.first().copied().unwrap_or(0)
                    }
                }
                ComponentKind::And(config) => evaluate_and(&config, &wired),
            };
            self.components[index].output = next;
        }
        let mut live = Live {
            inputs,
            ..Live::dark()
        };
        for face in Face::ALL {
            let Some((component, config)) = self.face_owner(face) else {
                continue;
            };
            if config.input {
                live.active[face.index()] = inputs[face.index()] > 0;
            } else {
                let power = io_emitted(&config, component.output);
                live.power[face.index()] = power;
                live.active[face.index()] = power > 0;
            }
        }
        live
    }

    /// The signal each face emits, from the last evaluation.
    #[must_use]
    pub fn power(&self) -> [u8; FACE_COUNT] {
        let mut power = [0u8; FACE_COUNT];
        for face in Face::ALL {
            if let Some((component, config)) = self.face_owner(face)
                && !config.input
            {
                power[face.index()] = io_emitted(&config, component.output);
            }
        }
        power
    }

    /// The component index and configuration of the first port (by slot) on
    /// `face`, if the face is not conflicted. Upstream's `load` registers one
    /// port per face and skips the rest.
    fn face_owner(&self, face: Face) -> Option<(Component, IoConfig)> {
        if self.io_conflicts().contains(&face) {
            return None;
        }
        self.components
            .iter()
            .filter(|component| {
                matches!(component.kind, ComponentKind::Io(config) if config.face == face)
            })
            .min_by_key(|component| component.slot)
            .and_then(|component| match component.kind {
                ComponentKind::Io(config) => Some((*component, config)),
                ComponentKind::And(_) => None,
            })
    }

    /// The signal each of the component's input ports receives: the first wire
    /// into the port whose source is above zero, else zero (`Microchip.tickLogic`).
    fn port_inputs(&self, component: Component) -> Vec<u8> {
        let mut wires: Vec<&Wire> = self.wires.iter().collect();
        wires.sort();
        (0..component.kind.input_ports())
            .map(|index| {
                let port = Port {
                    slot: component.slot,
                    index,
                };
                wires
                    .iter()
                    .filter(|wire| wire.input == port)
                    .find_map(|wire| {
                        let signal = self.output_of(wire.output);
                        (signal > 0).then_some(signal)
                    })
                    .unwrap_or(0)
            })
            .collect()
    }

    /// The stored output of the component at `port`, zero when there is none.
    fn output_of(&self, port: Port) -> u8 {
        self.components
            .iter()
            .find(|component| component.slot == port.slot)
            .map_or(0, |component| component.output)
    }

    fn index_of(&self, slot: i32) -> Option<usize> {
        self.components
            .iter()
            .position(|component| component.slot == slot)
    }
}

/// An AND gate's output for its inputs: zero if any input is zero, otherwise the
/// greatest input (`ANDGate.processInputs`, and the guide's analog note).
#[must_use]
pub fn evaluate_and(config: &AndConfig, inputs: &[u8]) -> u8 {
    if inputs.len() != usize::from(config.inputs) {
        return 0;
    }
    let mut greatest = 0;
    for input in inputs {
        if *input == 0 {
            return 0;
        }
        if *input > greatest {
            greatest = *input;
        }
    }
    greatest
}

/// An input port's output: the incoming signal when it meets the port's
/// comparison, otherwise zero (`LogicIO.processTickInternal`).
#[must_use]
pub fn io_input(config: &IoConfig, signal: u8) -> u8 {
    if config.comparison.test(signal, config.signal_strength) {
        signal
    } else {
        0
    }
}

/// The power an output port puts on its face: nothing when it is off, the
/// configured strength when one is set, otherwise the signal it passes through
/// (`LogicIO.processTickInternal`).
#[must_use]
pub fn io_emitted(config: &IoConfig, output: u8) -> u8 {
    if config.input || output == 0 {
        return 0;
    }
    if config.signal_strength == 0 {
        output
    } else {
        config.signal_strength
    }
}

/// A command the port's configuration surface accepts. Upstream edits a circuit
/// through the microchip's GUI; this port offers the same operations on the
/// command line (a documented deviation).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Print the usage.
    Help,
    /// Build the guide's AND scene on the microchip at `pos`.
    Configure {
        pos: [i32; 3],
        inputs: [Face; 2],
        output: Face,
    },
    /// Report the microchip at `pos` and its last live state.
    Status { pos: [i32; 3] },
    /// Empty the microchip at `pos`.
    Clear { pos: [i32; 3] },
    /// A command that is not this mod's.
    Unknown,
}

/// Why a command line was refused. Every variant carries what the player typed,
/// so the reply names the offending token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandError {
    /// Too few or too many arguments.
    WrongArity,
    /// A coordinate that is not an integer.
    BadCoordinate(String),
    /// A face name outside `down|up|north|south|west|east`.
    UnknownFace(String),
    /// The same face named twice.
    DuplicateFace(Face),
    /// An action the slice does not implement.
    UnknownAction(String),
}

impl fmt::Display for CommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandError::WrongArity => write!(formatter, "wrong number of arguments"),
            CommandError::BadCoordinate(token) => write!(formatter, "{token} is not a coordinate"),
            CommandError::UnknownFace(token) => write!(
                formatter,
                "{token} is not a face (down, up, north, south, west, east)"
            ),
            CommandError::DuplicateFace(face) => {
                write!(formatter, "{} is named twice", face.name())
            }
            CommandError::UnknownAction(token) => write!(formatter, "{token} is not an action"),
        }
    }
}

impl std::error::Error for CommandError {}

/// The command root and its alias.
const ROOTS: [&str; 2] = ["lbr", "little_big_redstone"];

/// Parse a command line (with or without its leading slash).
///
/// # Errors
/// A [`CommandError`] naming what was wrong, for the reply.
pub fn parse_command(line: &str) -> Result<Command, CommandError> {
    let line = line.trim();
    let line = line.strip_prefix('/').unwrap_or(line);
    let tokens: Vec<&str> = line.split_whitespace().collect();
    let Some(root) = tokens.first() else {
        return Err(CommandError::WrongArity);
    };
    if !ROOTS.contains(root) {
        return Ok(Command::Unknown);
    }
    let Some(verb) = tokens.get(1) else {
        return Err(CommandError::WrongArity);
    };
    match *verb {
        "help" => {
            if tokens.len() == 2 {
                Ok(Command::Help)
            } else {
                Err(CommandError::WrongArity)
            }
        }
        "circuit" => {
            if tokens.len() < 5 {
                return Err(CommandError::WrongArity);
            }
            let pos = [
                parse_coordinate(tokens[2])?,
                parse_coordinate(tokens[3])?,
                parse_coordinate(tokens[4])?,
            ];
            let Some(action) = tokens.get(5) else {
                return Err(CommandError::WrongArity);
            };
            match *action {
                "status" => {
                    if tokens.len() == 6 {
                        Ok(Command::Status { pos })
                    } else {
                        Err(CommandError::WrongArity)
                    }
                }
                "clear" => {
                    if tokens.len() == 6 {
                        Ok(Command::Clear { pos })
                    } else {
                        Err(CommandError::WrongArity)
                    }
                }
                "and" => {
                    if tokens.len() != 9 {
                        return Err(CommandError::WrongArity);
                    }
                    let input_a = parse_face(tokens[6])?;
                    let input_b = parse_face(tokens[7])?;
                    let output = parse_face(tokens[8])?;
                    if input_a == input_b {
                        return Err(CommandError::DuplicateFace(input_a));
                    }
                    if output == input_a || output == input_b {
                        return Err(CommandError::DuplicateFace(output));
                    }
                    Ok(Command::Configure {
                        pos,
                        inputs: [input_a, input_b],
                        output,
                    })
                }
                other => Err(CommandError::UnknownAction(other.to_owned())),
            }
        }
        _ => Err(CommandError::UnknownAction((*verb).to_owned())),
    }
}

fn parse_coordinate(token: &str) -> Result<i32, CommandError> {
    token
        .parse::<i32>()
        .map_err(|_| CommandError::BadCoordinate(token.to_owned()))
}

fn parse_face(token: &str) -> Result<Face, CommandError> {
    Face::parse(token).ok_or_else(|| CommandError::UnknownFace(token.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The jar's own truth table for the AND gate
    /// (`guides/little_big_redstone/guide/logic/and_gate.md`): 0,0 → 0;
    /// 0,1 → 0; 1,0 → 0; 1,1 → 1.
    #[test]
    fn and_gate_truth_table_matches_the_guide() {
        let rows = [(0, 0, 0), (0, 1, 0), (1, 0, 0), (1, 1, 1)];
        for (a, b, want) in rows {
            assert_eq!(
                and_output(&[a, b]),
                want,
                "AND({a},{b}) should be {want} (the jar's truth table)"
            );
        }
    }

    /// The guide's analog rule: "AND Gates will yield an output signal equal to
    /// the greatest input signal" — with every input on
    /// (`ANDGate.processInputs`: zero if any input is zero, else the maximum).
    #[test]
    fn and_gate_yields_the_greatest_input_when_all_are_on() {
        assert_eq!(and_output(&[15, 5]), 15);
        assert_eq!(and_output(&[5, 15]), 15);
        assert_eq!(and_output(&[1, 1]), 1);
        assert_eq!(and_output(&[15, 0]), 0);
        assert_eq!(and_output(&[0, 15]), 0);
    }

    /// `MultiLogicGateConfig.inputPortsAllowed()` is `2..=10`, and the
    /// constructor clamps into it.
    #[test]
    fn and_gate_clamps_its_input_count_to_two_through_ten() {
        assert_eq!(AndConfig::new(0).inputs, 2);
        assert_eq!(AndConfig::new(1).inputs, 2);
        assert_eq!(AndConfig::new(2).inputs, 2);
        assert_eq!(AndConfig::new(7).inputs, 7);
        assert_eq!(AndConfig::new(10).inputs, 10);
        assert_eq!(AndConfig::new(11).inputs, 10);
        assert_eq!(AndConfig::new(200).inputs, 10);
        assert_eq!(AndConfig::default().inputs, 2, "the default is the minimum");
    }

    /// The guide's four-input scene (`and_gate.md`): every single input alone is
    /// dark, and all four together are on.
    #[test]
    fn and_gate_with_four_inputs_requires_all_four() {
        for only in 0..4 {
            let mut inputs = [0u8; 4];
            inputs[only] = 15;
            assert_eq!(and_output(&inputs), 0, "input {only} alone must stay dark");
        }
        assert_eq!(and_output(&[15, 15, 15, 15]), 15);
        assert_eq!(and_output(&[15, 15, 15, 0]), 0);
    }

    /// `LogicIO.processTickInternal`, input mode:
    /// `signalComparison.test(signal, signalStrength) ? signal : 0` — the port
    /// passes the incoming strength through, and only when it meets the
    /// comparison.
    #[test]
    fn io_input_port_passes_the_signal_when_it_meets_the_comparison() {
        let at_least_one = IoConfig {
            signal_strength: 1,
            ..IoConfig::default()
        };
        assert_eq!(io_input(&at_least_one, 0), 0);
        assert_eq!(io_input(&at_least_one, 1), 1);
        assert_eq!(io_input(&at_least_one, 15), 15);

        let exactly_five = IoConfig {
            signal_strength: 5,
            comparison: Comparison::Equal,
            ..IoConfig::default()
        };
        assert_eq!(io_input(&exactly_five, 4), 0);
        assert_eq!(io_input(&exactly_five, 5), 5);
        assert_eq!(io_input(&exactly_five, 6), 0);

        let at_most_five = IoConfig {
            signal_strength: 5,
            comparison: Comparison::LessThanOrEqual,
            ..IoConfig::default()
        };
        assert_eq!(io_input(&at_most_five, 4), 4);
        assert_eq!(io_input(&at_most_five, 5), 5);
        assert_eq!(io_input(&at_most_five, 6), 0);
        assert_eq!(
            io_input(&at_most_five, 0),
            0,
            "0 never passes (upstream: 0 is off)"
        );
    }

    /// `LogicIOConfig.DEFAULT`: input mode, north, strength 1, "at least", weak.
    #[test]
    fn io_config_defaults_match_upstream() {
        let config = IoConfig::default();
        assert!(config.input);
        assert_eq!(config.face, Face::North);
        assert_eq!(config.signal_strength, 1);
        assert_eq!(config.comparison, Comparison::GreaterThanOrEqual);
        assert_eq!(config.power_type, PowerType::Weak);
    }

    /// `LogicIO.processTickInternal`, output mode: the emitted strength is the
    /// configured one, `0` means "pass the input through", and an off output
    /// emits nothing.
    #[test]
    fn io_output_port_emits_its_configured_strength_or_passes_it() {
        let fixed = IoConfig {
            input: false,
            signal_strength: 15,
            ..IoConfig::default()
        };
        assert_eq!(io_emitted(&fixed, 5), 15);
        assert_eq!(io_emitted(&fixed, 0), 0);

        let pass = IoConfig {
            input: false,
            signal_strength: 0,
            ..IoConfig::default()
        };
        assert_eq!(io_emitted(&pass, 5), 5);
        assert_eq!(io_emitted(&pass, 0), 0);

        let input_port = IoConfig::default();
        assert_eq!(io_emitted(&input_port, 9), 0, "an input port emits nothing");
    }

    /// `LogicIOConfig.checkValid`: an input and an output port on the same face
    /// make both inert ("If I/O ports are placed ... one is an input and the
    /// other is an output on the same face - neither port will work",
    /// `logic/io.md`).
    #[test]
    fn an_input_and_an_output_port_on_one_face_are_inert() {
        let mut circuit = Circuit::with_and_gate(Face::North, Face::South, Face::East)
            .expect("two distinct inputs and one output");
        // Add an output port on the north face, where input A already sits.
        circuit.push_component(Component {
            slot: 9,
            kind: ComponentKind::Io(IoConfig {
                input: false,
                face: Face::North,
                signal_strength: 15,
                ..IoConfig::default()
            }),
            output: 0,
        });
        let live = circuit.evaluate(signals(&[(Face::North, 15)]));
        assert_eq!(
            live.power[Face::North.index()],
            0,
            "the conflicting pair emits nothing"
        );
        assert!(
            !live.active[Face::North.index()],
            "and neither port reports the face live"
        );
        assert!(circuit.io_conflicts().contains(&Face::North));
    }

    /// Upstream `RedstoneAwareness.load` registers one port per face and skips
    /// the rest, so a second output port on a used face never drives it.
    #[test]
    fn the_first_port_on_a_face_owns_it() {
        let mut circuit = Circuit::new();
        circuit.push_component(Component {
            slot: 0,
            kind: ComponentKind::Io(IoConfig {
                face: Face::North,
                ..IoConfig::default()
            }),
            output: 0,
        });
        circuit.push_component(Component {
            slot: 3,
            kind: ComponentKind::Io(IoConfig {
                input: false,
                face: Face::East,
                signal_strength: 15,
                ..IoConfig::default()
            }),
            output: 0,
        });
        circuit.push_component(Component {
            slot: 1,
            kind: ComponentKind::Io(IoConfig {
                input: false,
                face: Face::East,
                signal_strength: 9,
                ..IoConfig::default()
            }),
            output: 0,
        });
        for slot in [1, 3] {
            circuit.push_wire(Wire {
                output: Port { slot: 0, index: 0 },
                input: Port { slot, index: 0 },
            });
        }
        // Slot 1 comes first, so its strength (9) is the one on the face.
        assert_eq!(
            circuit.evaluate(signals(&[(Face::North, 15)])).power[Face::East.index()],
            9
        );
    }

    /// A wire from an input port to the gate carries the face's signal; the
    /// gate's output reaches the output port on the east face.
    #[test]
    fn a_wire_carries_a_signal_from_a_face_to_the_output() {
        let mut circuit =
            Circuit::with_and_gate(Face::North, Face::South, Face::East).expect("distinct faces");
        let dark = circuit.evaluate([0; 6]);
        assert_eq!(dark.power[Face::East.index()], 0);
        assert_eq!(dark.active, [false; 6]);

        let one = circuit.evaluate(signals(&[(Face::North, 15)]));
        assert_eq!(one.power[Face::East.index()], 0, "one input is not enough");
        assert!(
            one.active[Face::North.index()],
            "the powered input face is live"
        );

        let both = circuit.evaluate(signals(&[(Face::North, 15), (Face::South, 15)]));
        assert_eq!(both.power[Face::East.index()], 15);
        assert!(both.active[Face::East.index()], "the output face is live");
        assert!(both.active[Face::North.index()] && both.active[Face::South.index()]);

        let analog = circuit.evaluate(signals(&[(Face::North, 15), (Face::South, 5)]));
        assert_eq!(
            analog.power[Face::East.index()],
            15,
            "greatest input, as the guide says"
        );
    }

    /// A face with no port is never reported live, whatever signal arrives there.
    #[test]
    fn a_face_without_a_port_is_never_live() {
        let mut circuit =
            Circuit::with_and_gate(Face::North, Face::South, Face::East).expect("distinct faces");
        let live = circuit.evaluate(signals(&[(Face::West, 15), (Face::East, 15)]));
        assert_eq!(live.active, [false; 6]);
        assert_eq!(live.power, [0; 6]);
    }

    /// Components tick in dependency order, not in storage order: an IO port
    /// feeding a gate must be evaluated before the gate reads it.
    #[test]
    fn evaluation_ticks_a_component_before_its_dependents() {
        // Same circuit, built backwards on purpose (gate before its ports).
        let mut circuit = Circuit::new();
        circuit.push_component(Component {
            slot: 9,
            kind: ComponentKind::Io(IoConfig {
                input: false,
                face: Face::East,
                signal_strength: 0,
                ..IoConfig::default()
            }),
            output: 0,
        });
        circuit.push_component(Component {
            slot: 5,
            kind: ComponentKind::And(AndConfig::new(2)),
            output: 0,
        });
        for (slot, face) in [(7, Face::North), (6, Face::South)] {
            circuit.push_component(Component {
                slot,
                kind: ComponentKind::Io(IoConfig {
                    face,
                    ..IoConfig::default()
                }),
                output: 0,
            });
        }
        circuit.push_wire(Wire {
            output: Port { slot: 7, index: 0 },
            input: Port { slot: 5, index: 0 },
        });
        circuit.push_wire(Wire {
            output: Port { slot: 6, index: 0 },
            input: Port { slot: 5, index: 1 },
        });
        circuit.push_wire(Wire {
            output: Port { slot: 5, index: 0 },
            input: Port { slot: 9, index: 0 },
        });
        let expected: Vec<usize> = [6, 7, 5, 9]
            .into_iter()
            .map(|slot| slot_index(&circuit, slot))
            .collect();
        assert_eq!(circuit.order(), expected);
        let live = circuit.evaluate(signals(&[(Face::North, 15), (Face::South, 15)]));
        assert_eq!(live.power[Face::East.index()], 15);
    }

    /// Evaluating twice with the same inputs gives the same answer: the stored
    /// component outputs are a cache, not hidden state.
    #[test]
    fn evaluation_is_idempotent() {
        let mut circuit =
            Circuit::with_and_gate(Face::North, Face::South, Face::East).expect("distinct faces");
        let both = signals(&[(Face::North, 15), (Face::South, 15)]);
        let first = circuit.evaluate(both);
        let second = circuit.evaluate(both);
        assert_eq!(first, second);
        let off = circuit.evaluate([0; 6]);
        assert_eq!(off.power, [0; 6]);
    }

    /// The slice's circuit is exactly the guide's own AND scene: two input
    /// ports, the gate, one output port, three wires.
    #[test]
    fn the_slice_circuit_matches_the_guide_scene() {
        let circuit =
            Circuit::with_and_gate(Face::North, Face::South, Face::East).expect("distinct faces");
        assert_eq!(circuit.components().len(), 4);
        assert_eq!(circuit.wires().len(), 3);
        let kinds: Vec<&str> = circuit
            .components()
            .iter()
            .map(|component| match component.kind {
                ComponentKind::Io(_) => "io",
                ComponentKind::And(_) => "and",
            })
            .collect();
        assert_eq!(kinds, ["io", "io", "and", "io"]);
        assert!(!circuit.is_empty());
        assert!(Circuit::new().is_empty());
    }

    /// The command surface replaces upstream's circuit-board GUI: it configures
    /// the microchip at a position, or reports it.
    #[test]
    fn the_command_parses_the_slice_surface() {
        assert_eq!(
            parse_command("lbr circuit 0 64 0 and north south east"),
            Ok(Command::Configure {
                pos: [0, 64, 0],
                inputs: [Face::North, Face::South],
                output: Face::East,
            })
        );
        assert_eq!(
            parse_command("/lbr circuit -3 70 12 status"),
            Ok(Command::Status { pos: [-3, 70, 12] })
        );
        assert_eq!(
            parse_command("lbr circuit 1 2 3 clear"),
            Ok(Command::Clear { pos: [1, 2, 3] })
        );
        assert_eq!(parse_command("lbr help"), Ok(Command::Help));
        assert_eq!(
            parse_command("other circuit 0 64 0 status"),
            Ok(Command::Unknown)
        );
    }

    /// Every wrong input combination is refused with a message, not accepted.
    #[test]
    fn the_command_refuses_bad_input() {
        assert_eq!(
            parse_command("lbr circuit 0 64 0 and north north east"),
            Err(CommandError::DuplicateFace(Face::North))
        );
        assert_eq!(
            parse_command("lbr circuit 0 64 0 and north south sideways"),
            Err(CommandError::UnknownFace("sideways".to_owned()))
        );
        assert_eq!(
            parse_command("lbr circuit 0 64 0 and north south"),
            Err(CommandError::WrongArity)
        );
        assert_eq!(
            parse_command("lbr circuit 0 64 0 flip north south east"),
            Err(CommandError::UnknownAction("flip".to_owned()))
        );
        assert_eq!(
            parse_command("lbr circuit zero 64 0 status"),
            Err(CommandError::BadCoordinate("zero".to_owned()))
        );
        assert_eq!(
            parse_command("lbr circuit 0 64 0 status extra"),
            Err(CommandError::WrongArity)
        );
        assert_eq!(parse_command("lbr"), Err(CommandError::WrongArity));
    }

    /// Every face name the guide lists is accepted, and only those.
    #[test]
    fn face_names_match_the_guide() {
        for (name, face) in [
            ("down", Face::Down),
            ("up", Face::Up),
            ("north", Face::North),
            ("south", Face::South),
            ("west", Face::West),
            ("east", Face::East),
        ] {
            assert_eq!(Face::parse(name), Some(face));
            assert_eq!(face.name(), name);
        }
        assert_eq!(Face::parse("Sideways"), None);
        assert_eq!(Face::parse(""), None);
        assert_eq!(Face::North.opposite(), Face::South);
        assert_eq!(Face::Up.opposite(), Face::Down);
        assert_eq!(Face::East.opposite(), Face::West);
    }

    /// `with_and_gate` refuses a layout whose faces collide, with the face named.
    #[test]
    fn a_circuit_cannot_put_two_ports_on_one_face() {
        assert_eq!(
            Circuit::with_and_gate(Face::North, Face::North, Face::East),
            Err(CommandError::DuplicateFace(Face::North))
        );
        assert_eq!(
            Circuit::with_and_gate(Face::North, Face::South, Face::South),
            Err(CommandError::DuplicateFace(Face::South))
        );
    }

    /// A wire whose source output is zero leaves the input port at zero, and a
    /// second wire into one port only wins when the first carries nothing
    /// (`Microchip.tickLogic`: the first wire with a signal above zero).
    #[test]
    fn a_zero_signal_wire_does_not_block_a_later_one() {
        let mut circuit =
            Circuit::with_and_gate(Face::North, Face::South, Face::East).expect("distinct faces");
        // A third input port on the west face wired to the gate's port 0, after
        // the north port's wire. North is dark, so the west wire supplies it.
        circuit.push_component(Component {
            slot: 8,
            kind: ComponentKind::Io(IoConfig {
                face: Face::West,
                ..IoConfig::default()
            }),
            output: 0,
        });
        circuit.push_wire(Wire {
            output: Port { slot: 8, index: 0 },
            input: Port { slot: 2, index: 0 },
        });
        let live = circuit.evaluate(signals(&[(Face::South, 15), (Face::West, 15)]));
        assert_eq!(
            live.power[Face::East.index()],
            15,
            "the west wire supplies the gate when north is dark"
        );
    }

    /// A per-face signal array from `(face, strength)` pairs, in ordinal order.
    fn signals(pairs: &[(Face, u8)]) -> [u8; FACE_COUNT] {
        let mut inputs = [0u8; FACE_COUNT];
        for (face, strength) in pairs {
            inputs[face.index()] = *strength;
        }
        inputs
    }

    fn and_output(inputs: &[u8]) -> u8 {
        let config = AndConfig::new(inputs.len() as u8);
        evaluate_and(&config, inputs)
    }

    fn slot_index(circuit: &Circuit, slot: i32) -> usize {
        circuit
            .components()
            .iter()
            .position(|component| component.slot == slot)
            .expect("the slot exists")
    }
}
