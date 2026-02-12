




#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Role {
    Farmer,
    Miner,
    Adventurer,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Farmer => "Farmer",
            Role::Miner => "Miner",
            Role::Adventurer => "Adventurer",
        }
    }
}