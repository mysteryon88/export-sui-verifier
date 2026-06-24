use std::{fs, path::PathBuf};

use anyhow::{bail, Context, Result};
use clap::Parser;
use sp1_sdk::{
    blocking::{ProveRequest, Prover, ProverClient},
    include_elf, Elf, HashableKey, ProvingKey, SP1Stdin,
};

const SIMPLE_SUM_ELF: Elf = include_elf!("simple-sum-program");

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[arg(long)]
    execute: bool,

    #[arg(long)]
    prove: bool,

    #[arg(long, default_value_t = 17)]
    a: u32,

    #[arg(long, default_value_t = 25)]
    b: u32,

    #[arg(long, default_value = "artifacts/simple_sum_proof.bin")]
    proof_out: PathBuf,

    #[arg(long, default_value = "artifacts/simple_sum_public_values.bin")]
    public_values_out: PathBuf,

    #[arg(long, default_value = "artifacts/simple_sum_program_vkey.txt")]
    program_vkey_out: PathBuf,

    #[arg(long, default_value = "artifacts/sp1_groth16_vk.bin")]
    wrapper_vk_out: PathBuf,
}

fn main() -> Result<()> {
    sp1_sdk::utils::setup_logger();

    let args = Args::parse();
    if args.execute == args.prove {
        bail!("pass exactly one of --execute or --prove");
    }

    let expected_sum = simple_sum_lib::checked_sum(args.a, args.b)
        .context("inputs overflow u32 addition before proving")?;
    let mut stdin = SP1Stdin::new();
    stdin.write(&args.a);
    stdin.write(&args.b);

    let client = ProverClient::from_env();

    if args.execute {
        let (public_values, report) = client.execute(SIMPLE_SUM_ELF, stdin).run()?;
        let (a, b, sum) = decode_public_values(public_values)?;
        anyhow::ensure!(
            (a, b, sum) == (args.a, args.b, expected_sum),
            "unexpected public values: got ({a}, {b}, {sum}), expected ({}, {}, {expected_sum})",
            args.a,
            args.b
        );

        println!("Executed simple-sum: {a} + {b} = {sum}");
        println!("Cycles: {}", report.total_instruction_count());
        return Ok(());
    }

    let pk = client.setup(SIMPLE_SUM_ELF)?;
    let proof = client.prove(&pk, stdin).groth16().run()?;
    client.verify(&proof, pk.verifying_key(), None)?;

    let (a, b, sum) = decode_public_values(proof.public_values.clone())?;
    anyhow::ensure!(
        (a, b, sum) == (args.a, args.b, expected_sum),
        "unexpected proof public values: got ({a}, {b}, {sum}), expected ({}, {}, {expected_sum})",
        args.a,
        args.b
    );

    write_parented(&args.proof_out, |path| proof.save(path))?;
    write_parented(&args.public_values_out, |path| {
        fs::write(path, proof.public_values.as_slice()).map_err(Into::into)
    })?;
    write_parented(&args.program_vkey_out, |path| {
        fs::write(path, format!("{}\n", pk.verifying_key().bytes32())).map_err(Into::into)
    })?;
    write_parented(&args.wrapper_vk_out, |path| {
        fs::write(path, &**sp1_verifier::GROTH16_VK_BYTES).map_err(Into::into)
    })?;

    println!("Generated and locally verified Groth16 proof for {a} + {b} = {sum}");
    println!("Proof: {}", args.proof_out.display());
    println!("Public values: {}", args.public_values_out.display());
    println!("Program vkey: {}", args.program_vkey_out.display());
    println!("SP1 Groth16 wrapper VK: {}", args.wrapper_vk_out.display());
    Ok(())
}

fn decode_public_values(mut public_values: sp1_sdk::SP1PublicValues) -> Result<(u32, u32, u32)> {
    let a = public_values.read::<u32>();
    let b = public_values.read::<u32>();
    let sum = public_values.read::<u32>();
    Ok((a, b, sum))
}

fn write_parented<T>(path: &PathBuf, write: impl FnOnce(&PathBuf) -> Result<T>) -> Result<T> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    write(path)
}
