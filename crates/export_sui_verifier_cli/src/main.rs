use clap::{Parser, Subcommand, ValueEnum};
use regex::Regex;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

use export_sui_verifier_core::curves::create_adapter;
use export_sui_verifier_core::error::{Error, Result};
use export_sui_verifier_core::formats::{
    load_arkworks_bundle_auto, load_arkworks_inputs_auto, load_gnark_binary_inputs_auto,
    load_gnark_json_inputs, load_snarkjs_json_inputs_with_optional_proof, load_sp1_groth16_inputs,
};
use export_sui_verifier_core::local_verify;
use export_sui_verifier_core::movegen::{
    generate_move_package, proof_data_snippet, GenerateMovePackageOptions, MovegenMode,
};

#[derive(Parser)]
#[command(
    name = "export-sui-verifier",
    version,
    about = "Export Groth16 artifacts to a Sui Move verifier package"
)]
struct Cli {
    #[command(flatten)]
    generate: GenerateArgs,
    #[command(subcommand)]
    command: Option<CliCommand>,
}

#[derive(Subcommand)]
enum CliCommand {
    ProofData(ProofDataArgs),
}

#[derive(clap::Args)]
struct GenerateArgs {
    #[arg(long)]
    vk: Option<PathBuf>,
    #[arg(long)]
    proof: Option<PathBuf>,
    #[arg(long)]
    public: Option<PathBuf>,
    #[arg(long)]
    bundle: Option<PathBuf>,
    #[arg(long)]
    out: Option<PathBuf>,
    #[arg(long)]
    package_name: Option<String>,
    #[arg(long)]
    module_name: Option<String>,

    #[arg(long, default_value_t = ModeArg::Entry)]
    mode: ModeArg,
    #[arg(long, default_value_t = false)]
    run_sui_test: bool,
    #[arg(long, default_value_t = false)]
    force: bool,
    #[arg(long, default_value_t = false)]
    skip_local_verify: bool,
}

#[derive(clap::Args)]
struct ProofDataArgs {
    #[arg(long)]
    vk: Option<PathBuf>,
    #[arg(long)]
    proof: Option<PathBuf>,
    #[arg(long)]
    public: Option<PathBuf>,
    #[arg(long)]
    bundle: Option<PathBuf>,

    #[arg(long, default_value_t = false)]
    skip_local_verify: bool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum ModeArg {
    Library,
    Entry,
    Test,
}

impl ModeArg {
    fn into_move_mode(self) -> MovegenMode {
        match self {
            Self::Library => MovegenMode::Library,
            Self::Entry => MovegenMode::Entry,
            Self::Test => MovegenMode::Test,
        }
    }
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Some(CliCommand::ProofData(args)) => run_proof_data(args),
        None => run_generate(cli.generate),
    };
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run_proof_data(args: ProofDataArgs) -> Result<()> {
    let inputs = load_inputs(
        args.vk.as_ref(),
        args.proof.as_ref(),
        args.public.as_ref(),
        args.bundle.as_ref(),
    )?;
    let requested_curve = inputs.curve.canonical_name().to_string();
    let adapter = create_adapter(&requested_curve)?;

    if !args.skip_local_verify && inputs.has_test_vectors() {
        let ok = local_verify(adapter.as_ref(), &inputs)?;
        if !ok {
            return Err(Error::LocalProofVerificationFailed(
                "local arkworks verification returned false".to_string(),
            ));
        }
    }

    let snippet = proof_data_snippet(adapter.as_ref(), &inputs)?;
    println!("{}", snippet.render_sui_test_functions());
    Ok(())
}

fn run_generate(args: GenerateArgs) -> Result<()> {
    let GenerateArgs {
        vk,
        proof,
        public,
        bundle,
        out,
        package_name,
        module_name,
        mode,
        run_sui_test: should_run_sui_test,
        force,
        skip_local_verify,
    } = args;

    let out =
        out.ok_or_else(|| Error::MissingInput("--out is required for generation".to_string()))?;
    let package_name = match package_name {
        Some(package_name) => package_name,
        None => default_package_name(&out)?,
    };
    let module_name = module_name.unwrap_or_else(|| "verifier".to_string());

    validate_names(&package_name, "package_name")?;
    validate_names(&module_name, "module_name")?;

    let inputs = load_inputs(
        vk.as_ref(),
        proof.as_ref(),
        public.as_ref(),
        bundle.as_ref(),
    )?;
    let requested_curve = inputs.curve.canonical_name().to_string();
    let adapter = create_adapter(&requested_curve)?;

    if !skip_local_verify && inputs.has_test_vectors() {
        let ok = local_verify(adapter.as_ref(), &inputs)?;
        if !ok {
            return Err(Error::LocalProofVerificationFailed(
                "local arkworks verification returned false".to_string(),
            ));
        }
    }

    generate_move_package(
        &out,
        adapter.as_ref(),
        &inputs,
        &GenerateMovePackageOptions {
            package_name: &package_name,
            module_name: &module_name,
            mode: mode.into_move_mode(),
            force,
        },
    )?;

    if should_run_sui_test {
        run_sui_test(&out)?;
    }

    Ok(())
}

fn load_inputs(
    vk: Option<&PathBuf>,
    proof: Option<&PathBuf>,
    public: Option<&PathBuf>,
    bundle: Option<&PathBuf>,
) -> Result<export_sui_verifier_core::model::Groth16VerifierInputs> {
    let inputs = match (bundle, vk) {
        (Some(bundle), None) => load_arkworks_bundle_auto(bundle)?,
        (None, Some(vk)) => load_auto_vk_inputs(
            vk,
            proof.map(PathBuf::as_path),
            public.map(PathBuf::as_path),
        )?,
        (Some(_), Some(_)) => {
            return Err(Error::MissingInput(
                "use either --bundle or --vk, not both".to_string(),
            ));
        }
        (None, None) => {
            return Err(Error::MissingInput(
                "--vk is required unless --bundle is used".to_string(),
            ));
        }
    };

    Ok(inputs)
}

fn load_auto_vk_inputs(
    vk: &Path,
    proof: Option<&Path>,
    public: Option<&Path>,
) -> Result<export_sui_verifier_core::model::Groth16VerifierInputs> {
    match load_snarkjs_json_inputs_with_optional_proof(vk, proof, public, None) {
        Ok(inputs) => Ok(inputs),
        Err(snarkjs_err) => match load_gnark_json_inputs(vk, proof, public, None) {
            Ok(inputs) => Ok(inputs),
            Err(gnark_json_err) => match load_arkworks_inputs_auto(vk, proof, public) {
                Ok(inputs) => Ok(inputs),
                Err(arkworks_err) => match load_gnark_binary_inputs_auto(vk, proof, public) {
                    Ok(inputs) => Ok(inputs),
                    Err(gnark_binary_err) => match proof {
                        Some(proof) if public.is_none() => match load_sp1_groth16_inputs(vk, proof)
                        {
                            Ok(inputs) => Ok(inputs),
                            Err(sp1_err) => Err(Error::MissingInput(format!(
                                "could not auto-detect artifact type: snarkjs failed with {snarkjs_err}; gnark json failed with {gnark_json_err}; arkworks failed with {arkworks_err}; gnark binary failed with {gnark_binary_err}; sp1 failed with {sp1_err}"
                            ))),
                        },
                        _ => Err(Error::MissingInput(format!(
                            "could not auto-detect artifact type: snarkjs failed with {snarkjs_err}; gnark json failed with {gnark_json_err}; arkworks failed with {arkworks_err}; gnark binary failed with {gnark_binary_err}"
                        ))),
                    },
                },
            },
        },
    }
}

impl fmt::Display for ModeArg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Library => "library",
            Self::Entry => "entry",
            Self::Test => "test",
        };
        write!(f, "{s}")
    }
}

fn default_package_name(out: &Path) -> Result<String> {
    let raw = out
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            Error::InvalidPackageName("--out must end with a package directory".to_string())
        })?;
    let mut name = String::with_capacity(raw.len());
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            name.push(ch);
        } else {
            name.push('_');
        }
    }
    if name.is_empty() {
        return Err(Error::InvalidPackageName(
            "--out must end with a non-empty package directory".to_string(),
        ));
    }
    if name.as_bytes()[0].is_ascii_digit() {
        name.insert(0, '_');
    }
    Ok(name)
}

fn validate_names(value: &str, field: &str) -> Result<()> {
    let re = Regex::new(r"^[A-Za-z_][A-Za-z0-9_]*$").unwrap();
    if !re.is_match(value) {
        if field == "module_name" {
            return Err(Error::InvalidModuleName(format!(
                "{field} must match [A-Za-z_][A-Za-z0-9_]*"
            )));
        }
        return Err(Error::InvalidPackageName(format!(
            "{field} must match [A-Za-z_][A-Za-z0-9_]*"
        )));
    }
    Ok(())
}

fn run_sui_test(out_dir: &std::path::Path) -> Result<()> {
    let sui = ProcessCommand::new("sui")
        .arg("move")
        .arg("test")
        .current_dir(out_dir)
        .output();

    match sui {
        Ok(out) => {
            if !out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                return Err(Error::SuiTestFailed(format!(
                    "ERR_SUI_TEST_FAILED: {}\nstdout:\n{}\nstderr:\n{}",
                    out.status, stdout, stderr
                )));
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Err(Error::SuiTestFailed(
                "ERR_SUI_CLI_NOT_FOUND: install Sui CLI or run without --run-sui-test".to_string(),
            ));
        }
        Err(err) => {
            return Err(Error::SuiTestFailed(err.to_string()));
        }
    }

    Ok(())
}
