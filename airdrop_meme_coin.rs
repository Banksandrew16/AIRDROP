use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, msg, pubkey::Pubkey,
    program_error::ProgramError, program_pack::Pack, sysvar::Sysvar,
};

// Define your token struct based on your token's data structure
#[derive(Debug)]
struct MyToken {
    pub balance: u64,
}

impl MyToken {
    fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let token = Self::try_from_slice(input)?;
        Ok(token)
    }
}

// Define the entrypoint for the airdrop
#[entrypoint]
fn airdrop_tokens(
    ctx: Context,
    #[account(mut, signer)] authority: AccountInfo,
    #[account(address = token_program)] token_program: AccountInfo,
    wallets: Vec<(Pubkey, u64)>, // List of wallet addresses and corresponding token amounts
) -> ProgramResult {
    // Verify that the authority is the expected signer
    if !authority.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load the token account associated with the authority
    let authority_token_account = ctx.accounts.authority_token_account.clone();

    // Initialize the token account if it's not already initialized
    if authority_token_account.is_uninitialized() {
        let mint_pubkey = *authority_token_account.mint;
        let initialize_account_ix = spl_token::instruction::initialize_account(
            &spl_token::id(),
            authority_token_account.key,
            mint_pubkey,
            authority.key,
        )?;
        msg!("Initializing token account for authority");
        solana_program::program::invoke(&initialize_account_ix, &[authority_token_account.clone(), authority.clone(), token_program.clone()])?;
    }

    // Iterate through the list of wallets and distribute tokens
    for (wallet_address, amount) in wallets {
        // Load the token account associated with the wallet address
        let wallet_token_account = ctx.accounts.wallet_token_accounts.get(&wallet_address).ok_or(ProgramError::InvalidArgument)?;

        // Transfer tokens from the authority's token account to the wallet's token account
        let transfer_ix = spl_token::instruction::transfer(
            &spl_token::id(),
            authority_token_account.key,
            wallet_token_account.key,
            authority.key,
            &[],
            amount,
        )?;
        msg!("Transferring tokens to wallet {}", wallet_address);
        solana_program::program::invoke(&transfer_ix, &[authority_token_account.clone(), wallet_token_account.clone(), authority.clone(), token_program.clone()])?;
    }

    Ok(())
}

// Define the program context
#[derive(Accounts)]
pub struct Context<'info> {
    #[account(mut, signer)]
    authority: AccountInfo<'info>,
    #[account(address = spl_token::ID)]
    token_program: AccountInfo<'info>,
    #[account(address = "<authority's token account address>")]
    authority_token_account: AccountInfo<'info>,
    #[account(address = "<list of wallet token accounts addresses>")]
    wallet_token_accounts: Vec<AccountInfo<'info>>,
    #[account(mut)]
    sysvar: Sysvar<'info, Rent>,
}

// Entry point for the airdrop program
#[cfg(not(feature = "no-entrypoint"))]
pub fn entrypoint(program_id: &Pubkey, accounts: &mut [AccountInfo], instruction_data: &[u8]) -> ProgramResult {
    // Deserialize the instruction data
    let wallets = deserialize_wallets(instruction_data)?;

    // Process the airdrop with the provided wallets
    airdrop_tokens(Context {
        authority: accounts[0].clone(),
        token_program: accounts[1].clone(),
        authority_token_account: accounts[2].clone(),
        wallet_token_accounts: accounts[3..].to_vec(),
        sysvar: Sysvar::from_account_info(&accounts[4])?,
    }, wallets)
}

// Deserialize the list of wallet addresses and amounts from the instruction data
fn deserialize_wallets(instruction_data: &[u8]) -> Result<Vec<(Pubkey, u64)>, ProgramError> {
    let mut offset = 0;
    let mut wallets = Vec::new();

    while offset < instruction_data.len() {
        let wallet_address = Pubkey::new_from_array(instruction_data[offset..offset + 32].try_into().unwrap());
        offset += 32;

        let amount = u64::from_le_bytes(instruction_data[offset..offset + 8].try_into().unwrap());
        offset += 8;

        wallets.push((wallet_address, amount));
    }

    Ok(wallets)
}
