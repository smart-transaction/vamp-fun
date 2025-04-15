use anchor_lang::prelude::*;
use prost::Message;

declare_id!("BMqQ4vojaUAd4BYo9Jtnq87rrTjt36mQDe9PGMAruxw7");

mod constants;
mod event;
mod instructions;
mod state;

pub use constants::*;
use instructions::*;

pub mod vamp_fun {
    include!(concat!(env!("OUT_DIR"), "/vamp.fun.rs"));
}

use vamp_fun::TokenVampingInfoProto;
#[program]
pub mod solana_vamp_program {
    use super::*;

    pub fn create_token_mint(ctx: Context<Initialize>, vamping_data: Vec<u8>) -> Result<()> {
        let vamping_info = TokenVampingInfoProto::decode(&vamping_data[..]).unwrap();
        let merkle_root: [u8; 32] = vamping_info.merkle_root[..]
            .try_into()
            .expect("Merkle root should be 32 bytes");
        ctx.accounts.create_token_mint(
            merkle_root,
            vamping_info.token_name,
            vamping_info.token_symbol,
            vamping_info.token_uri.unwrap_or_default(),
            vamping_info.amount,
            vamping_info.decimal as u8,
            &ctx.bumps,
        )?;

        Ok(())
    }
}
