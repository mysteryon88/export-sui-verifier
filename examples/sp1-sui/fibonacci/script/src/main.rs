use std::{fs, path::PathBuf};

use alloy_sol_types::SolType;
use anyhow::{bail, Result};
use clap::Parser;
use fibonacci_lib::{fibonacci, PublicValuesStruct};
use sp1_sdk::{
    blocking::{ProveRequest, Prover, ProverClient},
    include_elf, Elf, HashableKey, ProvingKey, SP1PublicValues, SP1Stdin,
};

const FIBONACCI_ELF: Elf = include_elf!("fibonacci-program");

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[arg(long)]
    execute: bool,

    #[arg(long)]
    prove: bool,

    #[arg(long, default_value_t = 20)]
    n: u32,

    #[arg(long, default_value = "artifacts/fibonacci_sp1_6.elf")]
    elf_out: PathBuf,

    #[arg(long, default_value = "artifacts/fibonacci_sp1_6_proof.bin")]
    proof_out: PathBuf,

    #[arg(long, default_value = "artifacts/fibonacci_sp1_6_public_values.bin")]
    public_values_out: PathBuf,

    #[arg(long, default_value = "artifacts/fibonacci_sp1_6_program_vkey.txt")]
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

    let expected = expected_public_values(args.n);
    let mut stdin = SP1Stdin::new();
    stdin.write(&args.n);

    let client = ProverClient::from_env();

    if args.execute {
        let (public_values, report) = client.execute(FIBONACCI_ELF, stdin).run()?;
        let decoded = decode_public_values(&public_values)?;
        ensure_expected(&decoded, &expected)?;

        println!(
            "Executed fibonacci: n={}, a={}, b={}",
            expected.n, expected.a, expected.b
        );
        println!("Cycles: {}", report.total_instruction_count());
        return Ok(());
    }

    let pk = client.setup(FIBONACCI_ELF)?;
    let proof = client.prove(&pk, stdin).groth16().run()?;
    client.verify(&proof, pk.verifying_key(), None)?;

    let decoded = decode_public_values(&proof.public_values)?;
    ensure_expected(&decoded, &expected)?;

    write_parented(&args.elf_out, |path| {
        fs::write(path, &*FIBONACCI_ELF).map_err(Into::into)
    })?;
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

    println!(
        "Generated and locally verified Groth16 proof for fibonacci n={}, a={}, b={}",
        expected.n, expected.a, expected.b
    );
    println!("ELF: {}", args.elf_out.display());
    println!("Proof: {}", args.proof_out.display());
    println!("Public values: {}", args.public_values_out.display());
    println!("Program vkey: {}", args.program_vkey_out.display());
    println!("SP1 Groth16 wrapper VK: {}", args.wrapper_vk_out.display());
    Ok(())
}

fn expected_public_values(n: u32) -> PublicValuesStruct {
    let (a, b) = fibonacci(n);
    PublicValuesStruct { n, a, b }
}

fn decode_public_values(public_values: &SP1PublicValues) -> Result<PublicValuesStruct> {
    Ok(PublicValuesStruct::abi_decode(public_values.as_slice())?)
}

fn ensure_expected(actual: &PublicValuesStruct, expected: &PublicValuesStruct) -> Result<()> {
    anyhow::ensure!(
        actual.n == expected.n && actual.a == expected.a && actual.b == expected.b,
        "unexpected public values: got n={}, a={}, b={}; expected n={}, a={}, b={}",
        actual.n,
        actual.a,
        actual.b,
        expected.n,
        expected.a,
        expected.b
    );
    Ok(())
}

fn write_parented<T>(path: &PathBuf, write: impl FnOnce(&PathBuf) -> Result<T>) -> Result<T> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    write(path)
}
