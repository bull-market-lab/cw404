use crate::{
    error::ContractError,
    state::{CURRENT_NFT_SUPPLY, NFTS, NFT_BALANCES, RECYCLED_NFT_IDS},
    util::assert::assert_max_nft_supply_not_reached,
};
use cosmwasm_std::{
    Addr, BlockInfo, Order, QuerierWrapper, StdError, StdResult, Storage,
    Uint128,
};
use cw404::nft::Nft;
use cw721::Approval;
use cw_utils::Expiration;

use super::assert::{assert_can_send, assert_can_update_approvals};

fn humanize_approval(approval: &Approval) -> Approval {
    Approval {
        spender: approval.clone().spender,
        expires: approval.expires,
    }
}
pub fn humanize_approvals(
    block: &BlockInfo,
    nft: &Nft,
    include_expired: bool,
) -> Vec<Approval> {
    nft.approvals
        .iter()
        .filter(|apr| include_expired || !apr.expires.is_expired(block))
        .map(humanize_approval)
        .collect()
}

pub fn parse_token_id_from_string_to_uint128(
    token_id: String,
) -> StdResult<Uint128> {
    let token_id_in_u128 = token_id
        .parse::<u128>()
        .map_err(|_| StdError::generic_err("token_id is not a valid u128"))?;
    Ok(Uint128::from(token_id_in_u128))
}

pub fn calculate_nft_to_mint_for_ft_mint(
    storage: &dyn Storage,
    querier: QuerierWrapper,
    owner_addr: &Addr,
    denom: String,
    ft_mint_amount: Uint128,
    one_denom_in_base_denom: Uint128,
) -> Result<Uint128, ContractError> {
    let before_ft_balance = querier.query_balance(owner_addr, denom)?.amount;
    let before_nft_balance = NFT_BALANCES.load(storage, owner_addr)?;
    let after_ft_balance = before_ft_balance + ft_mint_amount;
    let after_nft_balance = after_ft_balance / one_denom_in_base_denom;
    let mint_amount = after_nft_balance - before_nft_balance;
    // get before nft balance
    // get before ft balance
    // calculate after ft balance
    // calculate after nft balance
    // mint amount = after nft balance - before nft balance
    Ok(mint_amount)
}

pub fn calculate_nft_to_burn_for_ft_burn() -> Uint128 {
    Uint128::zero()
}

pub fn batch_mint_nft(
    storage: &mut dyn Storage,
    owner_addr: &Addr,
    max_denom_supply: Uint128,
    amount: Uint128,
) -> Result<(), ContractError> {
    let current_nft_supply = CURRENT_NFT_SUPPLY.load(storage)?;
    assert_max_nft_supply_not_reached(
        current_nft_supply,
        max_denom_supply,
        Uint128::one(),
    )?;

    for _ in 0..amount.into() {
        let token_id = if RECYCLED_NFT_IDS.len(storage)? > 0 {
            RECYCLED_NFT_IDS.pop_front(storage)?.unwrap()
        } else {
            // token_id starts from 1, so when current_nft_supply is 0, the next token_id is 1
            current_nft_supply + Uint128::one()
        };
        NFTS().update(storage, token_id.u128(), |old| match old {
            Some(_) => Err(ContractError::TokenIdAlreadyInUse { token_id }),
            None => Ok(Nft {
                owner: owner_addr.clone(),
                approvals: vec![],
                token_uri: None,
            }),
        })?;
    }

    let updated_nft_supply = current_nft_supply + amount;
    CURRENT_NFT_SUPPLY.save(storage, &updated_nft_supply)?;

    Ok(())
}

pub fn batch_burn_nft(
    storage: &mut dyn Storage,
    owner_addr: &Addr,
    amount: Uint128,
) -> Result<(), ContractError> {
    let current_nft_supply = CURRENT_NFT_SUPPLY.load(storage)?;

    let token_ids: Vec<u128> = NFTS()
        .idx
        .owner
        .prefix(owner_addr.clone())
        .keys(storage, None, None, Order::Ascending)
        .take(amount.u128() as usize)
        .collect::<StdResult<Vec<_>>>()?;
    if token_ids.len() != amount.u128() as usize {
        return Err(ContractError::CannotBurnMoreNftThanOwned {
            available: Uint128::from(token_ids.len() as u128),
            try_to_burn: amount,
        });
    }

    for token_id in token_ids {
        NFTS().remove(storage, token_id)?;
    }
    let updated_nft_supply = current_nft_supply - amount;
    CURRENT_NFT_SUPPLY.save(storage, &updated_nft_supply)?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn update_approvals(
    storage: &mut dyn Storage,
    block: &BlockInfo,
    sender_addr: &Addr,
    spender_addr: &Addr,
    token_id: Uint128,
    // if add == false, remove. if add == true, remove then set with this expiration
    add: bool,
    expires: Option<Expiration>,
) -> Result<Nft, ContractError> {
    let mut nft = NFTS().load(storage, token_id.u128())?;
    // ensure we have permissions
    assert_can_update_approvals(storage, block, &nft.owner, sender_addr)?;

    // update the approval list (remove any for the same spender before adding)
    nft.approvals
        .retain(|apr| apr.spender != spender_addr.to_string());

    // only difference between approve and revoke
    if add {
        // reject expired data as invalid
        let expires = expires.unwrap_or_default();
        if expires.is_expired(block) {
            return Err(ContractError::Expired {});
        }
        let approval = Approval {
            spender: spender_addr.to_string(),
            expires,
        };
        nft.approvals.push(approval);
    }

    NFTS().save(storage, token_id.u128(), &nft)?;

    Ok(nft)
}

pub fn transfer_nft_helper(
    storage: &mut dyn Storage,
    block: &BlockInfo,
    sender_addr: &Addr,
    recipient_addr: &Addr,
    token_id: Uint128,
) -> Result<Nft, ContractError> {
    let mut nft = NFTS().load(storage, token_id.u128())?;
    // ensure we have permissions
    assert_can_send(storage, block, sender_addr, token_id)?;
    // set owner and remove existing approvals
    nft.owner = recipient_addr.clone();
    nft.approvals = vec![];
    NFTS().save(storage, token_id.u128(), &nft)?;
    Ok(nft)
}
