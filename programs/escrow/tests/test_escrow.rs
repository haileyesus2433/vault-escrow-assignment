use {
    anchor_lang::{
        solana_program::program_pack::Pack,
        system_program::ID as SYSTEM_PROGRAM_ID,
        AccountDeserialize, InstructionData, ToAccountMetas,
    },
    anchor_spl::{
        associated_token::{self, ID as ASSOCIATED_TOKEN_PROGRAM_ID},
        token::spl_token,
    },
    litesvm::LiteSVM,
    litesvm_token::{
        spl_token::ID as TOKEN_PROGRAM_ID,
        CreateAssociatedTokenAccount, CreateMint, MintTo,
    },
    solana_keypair::Keypair,
    solana_message::Message,
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    solana_transaction::Transaction,
    anchor_lang::solana_program::instruction::Instruction,
};

fn setup() -> (LiteSVM, Keypair) {
    let program_id = escrow::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../target/deploy/escrow.so");
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    (svm, payer)
}

#[test]
fn test_make_and_take() {
    let (mut svm, maker) = setup();
    let taker = Keypair::new();
    svm.airdrop(&taker.pubkey(), 10_000_000_000).unwrap();

    let mint_a = CreateMint::new(&mut svm, &maker)
        .decimals(6)
        .authority(&maker.pubkey())
        .send()
        .unwrap();

    let mint_b = CreateMint::new(&mut svm, &maker)
        .decimals(6)
        .authority(&maker.pubkey())
        .send()
        .unwrap();

    let maker_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint_a)
        .owner(&maker.pubkey())
        .send()
        .unwrap();

    let maker_ata_b = CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint_b)
        .owner(&maker.pubkey())
        .send()
        .unwrap();

    let taker_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &taker, &mint_a)
        .owner(&taker.pubkey())
        .send()
        .unwrap();

    let taker_ata_b = CreateAssociatedTokenAccount::new(&mut svm, &taker, &mint_b)
        .owner(&taker.pubkey())
        .send()
        .unwrap();

    MintTo::new(&mut svm, &maker, &mint_a, &maker_ata_a, 1_000_000_000)
        .send()
        .unwrap();

    MintTo::new(&mut svm, &maker, &mint_b, &taker_ata_b, 1_000_000_000)
        .send()
        .unwrap();

    let seed = 42u64;
    let (escrow_pda, bump) = Pubkey::find_program_address(
        &[b"escrow", maker.pubkey().as_ref(), &seed.to_le_bytes()],
        &escrow::id(),
    );

    let vault = associated_token::get_associated_token_address(&escrow_pda, &mint_a);

    let deposit_amount = 500_000_000;
    let receive_amount = 200_000_000;

    // 1. Make
    let make_ix = Instruction {
        program_id: escrow::id(),
        accounts: escrow::accounts::Make {
            maker: maker.pubkey(),
            mint_a,
            mint_b,
            maker_ata_a,
            escrow: escrow_pda,
            vault,
            associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            token_program: TOKEN_PROGRAM_ID,
            system_program: SYSTEM_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: escrow::instruction::Make {
            seed,
            deposit: deposit_amount,
            receive: receive_amount,
        }
        .data(),
    };

    let msg = Message::new(&[make_ix], Some(&maker.pubkey()));
    let tx = Transaction::new(&[&maker], msg, svm.latest_blockhash());
    svm.send_transaction(tx).unwrap();

    let vault_account = svm.get_account(&vault).unwrap();
    let vault_data = spl_token::state::Account::unpack(&vault_account.data).unwrap();
    assert_eq!(vault_data.amount, deposit_amount);

    let escrow_account = svm.get_account(&escrow_pda).unwrap();
    let escrow_data =
        escrow::state::Escrow::try_deserialize(&mut escrow_account.data.as_ref()).unwrap();
    assert_eq!(escrow_data.seed, seed);
    assert_eq!(escrow_data.receive, receive_amount);
    assert_eq!(escrow_data.bump, bump);

    // 2. Take
    let take_ix = Instruction {
        program_id: escrow::id(),
        accounts: escrow::accounts::Take {
            taker: taker.pubkey(),
            maker: maker.pubkey(),
            mint_a,
            mint_b,
            taker_ata_a,
            taker_ata_b,
            maker_ata_b,
            escrow: escrow_pda,
            vault,
            associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            token_program: TOKEN_PROGRAM_ID,
            system_program: SYSTEM_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: escrow::instruction::Take {}.data(),
    };

    let msg = Message::new(&[take_ix], Some(&taker.pubkey()));
    let tx = Transaction::new(&[&taker], msg, svm.latest_blockhash());
    svm.send_transaction(tx).unwrap();

    let taker_a = svm.get_account(&taker_ata_a).unwrap();
    let taker_a_data = spl_token::state::Account::unpack(&taker_a.data).unwrap();
    assert_eq!(taker_a_data.amount, deposit_amount);

    let maker_b = svm.get_account(&maker_ata_b).unwrap();
    let maker_b_data = spl_token::state::Account::unpack(&maker_b.data).unwrap();
    assert_eq!(maker_b_data.amount, receive_amount);

    assert!(svm.get_account(&escrow_pda).is_none());
    assert!(svm.get_account(&vault).is_none());
}

#[test]
fn test_make_and_refund() {
    let (mut svm, maker) = setup();

    let mint_a = CreateMint::new(&mut svm, &maker)
        .decimals(6)
        .authority(&maker.pubkey())
        .send()
        .unwrap();

    let mint_b = CreateMint::new(&mut svm, &maker)
        .decimals(6)
        .authority(&maker.pubkey())
        .send()
        .unwrap();

    let maker_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint_a)
        .owner(&maker.pubkey())
        .send()
        .unwrap();

    MintTo::new(&mut svm, &maker, &mint_a, &maker_ata_a, 1_000_000_000)
        .send()
        .unwrap();

    let seed = 999u64;
    let (escrow_pda, _bump) = Pubkey::find_program_address(
        &[b"escrow", maker.pubkey().as_ref(), &seed.to_le_bytes()],
        &escrow::id(),
    );

    let vault = associated_token::get_associated_token_address(&escrow_pda, &mint_a);

    let deposit_amount = 300_000_000;
    let receive_amount = 100_000_000;

    // 1. Make
    let make_ix = Instruction {
        program_id: escrow::id(),
        accounts: escrow::accounts::Make {
            maker: maker.pubkey(),
            mint_a,
            mint_b,
            maker_ata_a,
            escrow: escrow_pda,
            vault,
            associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            token_program: TOKEN_PROGRAM_ID,
            system_program: SYSTEM_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: escrow::instruction::Make {
            seed,
            deposit: deposit_amount,
            receive: receive_amount,
        }
        .data(),
    };

    let msg = Message::new(&[make_ix], Some(&maker.pubkey()));
    let tx = Transaction::new(&[&maker], msg, svm.latest_blockhash());
    svm.send_transaction(tx).unwrap();

    let vault_account = svm.get_account(&vault).unwrap();
    let vault_data = spl_token::state::Account::unpack(&vault_account.data).unwrap();
    assert_eq!(vault_data.amount, deposit_amount);

    // 2. Refund
    let refund_ix = Instruction {
        program_id: escrow::id(),
        accounts: escrow::accounts::Refund {
            maker: maker.pubkey(),
            mint_a,
            maker_ata_a,
            escrow: escrow_pda,
            vault,
            associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            token_program: TOKEN_PROGRAM_ID,
            system_program: SYSTEM_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: escrow::instruction::Refund {}.data(),
    };

    let msg = Message::new(&[refund_ix], Some(&maker.pubkey()));
    let tx = Transaction::new(&[&maker], msg, svm.latest_blockhash());
    svm.send_transaction(tx).unwrap();

    let maker_a = svm.get_account(&maker_ata_a).unwrap();
    let maker_a_data = spl_token::state::Account::unpack(&maker_a.data).unwrap();
    assert_eq!(maker_a_data.amount, 1_000_000_000);

    assert!(svm.get_account(&escrow_pda).is_none());
    assert!(svm.get_account(&vault).is_none());
}

#[test]
fn test_make_and_update() {
    let (mut svm, maker) = setup();

    let mint_a = CreateMint::new(&mut svm, &maker)
        .decimals(6)
        .authority(&maker.pubkey())
        .send()
        .unwrap();

    let mint_b = CreateMint::new(&mut svm, &maker)
        .decimals(6)
        .authority(&maker.pubkey())
        .send()
        .unwrap();

    let maker_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint_a)
        .owner(&maker.pubkey())
        .send()
        .unwrap();

    MintTo::new(&mut svm, &maker, &mint_a, &maker_ata_a, 1_000_000_000)
        .send()
        .unwrap();

    let seed = 7u64;
    let (escrow_pda, _bump) = Pubkey::find_program_address(
        &[b"escrow", maker.pubkey().as_ref(), &seed.to_le_bytes()],
        &escrow::id(),
    );

    let vault = associated_token::get_associated_token_address(&escrow_pda, &mint_a);

    let deposit_amount = 200_000_000;
    let receive_amount = 50_000_000;

    // 1. Make
    let make_ix = Instruction {
        program_id: escrow::id(),
        accounts: escrow::accounts::Make {
            maker: maker.pubkey(),
            mint_a,
            mint_b,
            maker_ata_a,
            escrow: escrow_pda,
            vault,
            associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            token_program: TOKEN_PROGRAM_ID,
            system_program: SYSTEM_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: escrow::instruction::Make {
            seed,
            deposit: deposit_amount,
            receive: receive_amount,
        }
        .data(),
    };

    let msg = Message::new(&[make_ix], Some(&maker.pubkey()));
    let tx = Transaction::new(&[&maker], msg, svm.latest_blockhash());
    svm.send_transaction(tx).unwrap();

    // Confirm initial receive amount
    let escrow_account = svm.get_account(&escrow_pda).unwrap();
    let escrow_data =
        escrow::state::Escrow::try_deserialize(&mut escrow_account.data.as_ref()).unwrap();
    assert_eq!(escrow_data.receive, receive_amount);

    // 2. Update receive amount
    let new_receive = 150_000_000;
    let update_ix = Instruction {
        program_id: escrow::id(),
        accounts: escrow::accounts::Update {
            maker: maker.pubkey(),
            escrow: escrow_pda,
        }
        .to_account_metas(None),
        data: escrow::instruction::Update {
            receive: new_receive,
        }
        .data(),
    };

    let msg = Message::new(&[update_ix], Some(&maker.pubkey()));
    let tx = Transaction::new(&[&maker], msg, svm.latest_blockhash());
    svm.send_transaction(tx).unwrap();

    // Confirm receive amount was updated
    let escrow_account = svm.get_account(&escrow_pda).unwrap();
    let escrow_data =
        escrow::state::Escrow::try_deserialize(&mut escrow_account.data.as_ref()).unwrap();
    assert_eq!(escrow_data.receive, new_receive);
}
