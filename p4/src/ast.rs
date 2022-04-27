use std::fmt;
use std::collections::BTreeMap;

use crate::lexer::Token;

#[derive(Debug)]
pub struct AST {
    pub constants: Vec<Constant>,
    pub headers: Vec<Header>,
    pub structs: Vec<Struct>,
    pub typedefs: Vec<Typedef>,
    pub controls: Vec<Control>,
    pub parsers: Vec<Parser>,
    pub packages: Vec<Package>,
    pub package_instance: Option<PackageInstance>,
}

impl Default for AST {
    fn default() -> Self {
        Self{
            constants: Vec::new(),
            headers: Vec::new(),
            structs: Vec::new(),
            typedefs: Vec::new(),
            controls: Vec::new(),
            parsers: Vec::new(),
            packages: Vec::new(),
            package_instance: None,
        }
    }
}

impl AST {

    pub fn get_struct(&self, name: &str) -> Option<&Struct> {
        for s in &self.structs {
            if s.name == name {
                return Some(s)
            }
        }
        None
    }

    pub fn get_header(&self, name: &str) -> Option<&Header> {
        for s in &self.headers {
            if s.name == name {
                return Some(s)
            }
        }
        None
    }
}

#[derive(Debug)]
pub struct PackageInstance {
    pub instance_type: String,
    pub name: String,
    pub parameters: Vec::<String>,
}

impl PackageInstance {
    pub fn new(instance_type: String) -> Self {
        Self{ instance_type, name: "".into(), parameters: Vec::new() }
    }
}

#[derive(Debug)]
pub struct Package {
    pub name: String,
    pub type_parameters: Vec::<String>,
    pub parameters: Vec::<PackageParameter>,
}

impl Package {
    pub fn new(name: String) -> Self {
        Self{
            name,
            type_parameters: Vec::new(),
            parameters: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct PackageParameter {
    pub type_name: String,
    pub type_parameters: Vec::<String>,
    pub name: String,
}

impl PackageParameter {
    pub fn new(type_name: String) -> Self {
        Self{
            type_name,
            type_parameters: Vec::new(),
            name: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Type {
    Bool,
    Error,
    Bit(usize),
    Varbit(usize),
    Int(usize),
    String,
    UserDefined(String),
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            Type::Bool => write!(f, "bool"),
            Type::Error => write!(f, "error"),
            Type::Bit(size) => write!(f, "bit<{}>", size),
            Type::Varbit(size) => write!(f, "varbit<{}>", size),
            Type::Int(size) => write!(f, "int<{}>", size),
            Type::String => write!(f, "string"),
            Type::UserDefined(name) => write!(f, "{}", name),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Typedef {
    pub ty: Type,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Constant {
    pub ty: Type,
    pub name: String,
    pub initializer: Box::<Expression>,
}

#[derive(Debug, Clone)]
pub struct Variable {
    pub ty: Type,
    pub name: String,
    //TODO initializer: Expression,
}

#[derive(Debug, Clone)]
pub enum Expression {
    IntegerLit(i128),
    BitLit(u16, u128),
    SignedLit(u16, i128),
    Lvalue(Lvalue),
    Binary(Box::<Expression>, BinOp, Box::<Expression>),
}

#[derive(Debug, Clone)]
pub enum BinOp {
    Add,
    Subtract,
    Geq,
    Eq,
}

#[derive(Debug, Clone)]
pub struct Header {
    pub name: String,
    pub members: Vec::<HeaderMember>,
}

impl Header {
    pub fn new(name: String) -> Self {
        Header{name,  members: Vec::new() }
    }
}

#[derive(Debug, Clone)]
pub struct HeaderMember {
    pub ty: Type,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Struct {
    pub name: String,
    pub members: Vec::<StructMember>,
}

impl Struct {
    pub fn new(name: String) -> Self {
        Struct{name,  members: Vec::new() }
    }
}

#[derive(Debug, Clone)]
pub struct StructMember {
    pub ty: Type,
    pub name: String,
    pub token: Token,
}

#[derive(Debug, Clone)]
pub struct Control {
    pub name: String,
    pub type_parameters: Vec::<String>,
    pub parameters: Vec::<ControlParameter>,
    pub actions: Vec::<Action>,
    pub tables: Vec::<Table>,
    pub apply: StatementBlock,
}

impl Control {
    pub fn new(name: String) -> Self {
        Self{
            name,
            type_parameters: Vec::new(),
            parameters: Vec::new(),
            actions: Vec::new(),
            tables: Vec::new(),
            apply: StatementBlock::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Parser {
    pub name: String,
    pub type_parameters: Vec::<String>,
    pub parameters: Vec::<ControlParameter>,
    pub states: Vec::<State>,

    /// The first token of this parser, used for error reporting.
    pub token: Token,
}

impl Parser {
    pub fn new(name: String, token: Token) -> Self {
        Self{
            name,
            type_parameters: Vec::new(),
            parameters: Vec::new(),
            states: Vec::new(),
            token,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ControlParameter {
    pub direction: Direction,
    pub ty: Type,
    pub name: String,

    /// The first token of this parser, used for error reporting.
    pub dir_token: Token,
    pub ty_token: Token,
    pub name_token: Token,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Direction {
    In,
    Out,
    InOut,
    Unspecified,
}

#[derive(Debug, Clone, Default)]
pub struct StatementBlock {
    pub variables: Vec::<Variable>,
    pub constants: Vec::<Constant>,
    pub statements: Vec::<Statement>,
}

#[derive(Debug, Clone)]
pub struct Action {
    pub name: String,
    pub parameters: Vec::<ActionParameter>,
    pub statement_block: StatementBlock,
}

impl Action {
    pub fn new(name: String) -> Self {
        Self{
            name,
            parameters: Vec::new(),
            statement_block: StatementBlock::default(),
        }
    }
}


#[derive(Debug, Clone)]
pub struct ActionParameter {
    pub ty: Type,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Table {
    pub name: String,
    pub actions: Vec::<String>,
    pub default_action: String,
    pub key: BTreeMap<Lvalue, MatchKind>,
    pub const_entries: Vec::<ConstTableEntry>,
    pub size: usize,
}

impl Table {
    pub fn new(name: String) -> Self {
        Self{
            name,
            actions: Vec::new(),
            default_action: String::new(),
            key: BTreeMap::new(),
            const_entries: Vec::new(),
            size: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConstTableEntry {
    pub keyset: Vec::<KeySetElement>,
    pub action: ActionRef,
}

#[derive(Debug, Clone)]
pub enum KeySetElement {
    Expression(Box::<Expression>),
    Default,
    DontCare,
    Masked(Box::<Expression>, Box::<Expression>),
    Ranged(Box::<Expression>, Box::<Expression>),
}


#[derive(Debug, Clone)]
pub struct ActionRef {
    pub name: String,
    pub parameters: Vec::<Box<Expression>>,
}

impl ActionRef {
    pub fn new(name: String) -> Self {
        Self { name, parameters: Vec::new() }
    }
}

#[derive(Debug, Clone)]
pub enum MatchKind {
    Exact,
    Ternary,
    LongestPrefixMatch,
}

#[derive(Debug, Clone)]
pub enum Statement {
    Empty,
    Assignment(Lvalue, Box::<Expression>),
    Call(Call),
    // TODO ...
}

/// A function or method call
#[derive(Debug, Clone)]
pub struct Call {
    pub lval: Lvalue,
    pub args: Vec::<Box::<Expression>>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Lvalue {
    pub name: String,
}


#[derive(Debug, Clone)]
pub struct State {
    pub name: String,
    pub variables: Vec::<Variable>,
    pub constants: Vec::<Constant>,
    pub statements: Vec::<Statement>,
    pub transition: Option<Transition>,
}

impl State {
    pub fn new(name: String) -> Self {
        Self{
            name,
            variables: Vec::new(),
            constants: Vec::new(),
            statements: Vec::new(),
            transition: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Transition {
    Reference(String),
    Select(Select),
}

#[derive(Debug, Clone, Default)]
pub struct Select {
    pub parameters: Vec::<Box::<Expression>>,
    pub elements: Vec::<SelectElement>,
}

#[derive(Debug, Clone)]
pub struct SelectElement {
    pub keyset: Vec::<KeySetElement>,
    pub name: String,
}