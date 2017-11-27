//! JSON generation

#[derive(Serialize, Debug)]
pub struct FunctionEntry;

#[derive(Serialize, Debug)]
pub struct ConstructorEntry;

#[derive(Serialize, Debug)]
#[serde(tag = "type")]
pub enum AbiEntry {
    #[serde(rename = "event")]
    Event(EventEntry),
    #[serde(rename = "function")]
    Function(FunctionEntry),
    #[serde(rename = "constructor")]
    Constructor(ConstructorEntry),
}

#[derive(Serialize, Debug)]
pub struct EventInput {
    pub name: String,
    pub type_: String,
    pub indexed: bool,
}

#[derive(Serialize, Debug)]
pub struct EventEntry {
    pub name: String,
    pub inputs: Vec<EventInput>,
}