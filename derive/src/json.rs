//! JSON generation

use {items, utils};

#[derive(Serialize, Debug)]
pub struct FunctionEntry {
    pub name: String,
    #[serde(rename = "inputs")]
    pub arguments: Vec<Argument>,
    pub outputs: Vec<Argument>,
    pub constant: bool,
}

#[derive(Serialize, Debug)]
pub struct Argument {
    pub name: String,
    #[serde(rename = "type")]
    pub type_: String,
}

#[derive(Serialize, Debug)]
pub struct ConstructorEntry {
    #[serde(rename = "inputs")]
    pub arguments: Vec<Argument>,
}

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
    #[serde(rename = "type")]
    pub type_: String,
    pub indexed: bool,
}

#[derive(Serialize, Debug)]
pub struct EventEntry {
    pub name: String,
    pub inputs: Vec<EventInput>,
}

#[derive(Serialize, Debug)]
pub struct Abi(pub Vec<AbiEntry>);

impl<'a> From<&'a items::Interface> for Abi {
    fn from(intf: &items::Interface) -> Self {
        let mut result = Vec::new();
        for item in intf.items() {
            match *item {
                items::Item::Event(ref event) => result.push(AbiEntry::Event(event.into())),
                items::Item::Signature(ref signature) => result.push(AbiEntry::Function(signature.into())),
                _ => {}
            }
        }

        if let Some(constructor) = intf.constructor() {
            result.push(AbiEntry::Constructor(FunctionEntry::from(constructor).into()));
        }

        Abi(result)
    }
}

impl<'a> From<&'a items::Event> for EventEntry {
    fn from(item: &items::Event) -> Self {
        EventEntry {
            name: item.name.to_string(),
            inputs: item.indexed
                .iter()
                .map(|&(ref pat, ref ty)|
                    EventInput {
                        name: quote! { #pat }.to_string(),
                        type_: utils::canonical_ty(ty),
                        indexed: true,
                    }
                )
                .chain(
                    item.data
                        .iter()
                        .map(|&(ref pat, ref ty)|
                            EventInput {
                                name: quote! { #pat }.to_string(),
                                type_: utils::canonical_ty(ty),
                                indexed: false,
                            }
                        )
                    )
                .collect(),
        }
    }
}

impl<'a> From<&'a items::Signature> for FunctionEntry {
    fn from(item: &items::Signature) -> Self {
        FunctionEntry {
            name: item.name.to_string(),
            arguments: item.arguments
                .iter()
                .map(|&(ref pat, ref ty)|
                    Argument {
                        name: quote! { #pat }.to_string(),
                        type_: utils::canonical_ty(ty),
                    }
                )
                .collect(),
            outputs: item.return_type
                .iter()
                .enumerate()
                .map(|(idx, ty)| Argument { name: format!("returnValue{}", idx), type_: utils::canonical_ty(ty) })
                .collect(),
            constant: item.is_constant,
        }
    }
}

impl From<FunctionEntry> for ConstructorEntry {
    fn from(func: FunctionEntry) -> Self {
        ConstructorEntry { arguments: func.arguments }
    }
}