use ark_bls12_381::{G1Affine as BlsG1Affine, G2Affine as BlsG2Affine};
use ark_bn254::{G1Affine as Bn254G1Affine, G2Affine as Bn254G2Affine};
use ark_serialize::CanonicalDeserialize;
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::Path;

use crate::curves::create_adapter;
use crate::error::{Error, Result};
use crate::model::{
    CurveKind, Groth16G1Point, Groth16G2Point, Groth16Proof, Groth16VerificationKey,
    Groth16VerifierInputs, SourceFormat,
};
use crate::snarkjs::parse_decimal;

#[derive(Debug, Deserialize)]
struct GnarkVerificationKeyJson {
    #[serde(rename = "G1")]
    g1: GnarkVkG1Json,
    #[serde(rename = "G2")]
    g2: GnarkVkG2Json,
    #[serde(rename = "CommitmentKeys", default)]
    commitment_keys: Option<Value>,
    #[serde(rename = "PublicAndCommitmentCommitted", default)]
    public_and_commitment_committed: Vec<Value>,
}

#[derive(Debug, Deserialize)]
struct GnarkVkG1Json {
    #[serde(rename = "Alpha")]
    alpha: GnarkG1Json,
    #[serde(rename = "K")]
    k: Vec<GnarkG1Json>,
}

#[derive(Debug, Deserialize)]
struct GnarkVkG2Json {
    #[serde(rename = "Beta")]
    beta: GnarkG2Json,
    #[serde(rename = "Gamma")]
    gamma: GnarkG2Json,
    #[serde(rename = "Delta")]
    delta: GnarkG2Json,
}

#[derive(Debug, Deserialize)]
struct GnarkProofJson {
    #[serde(rename = "Ar")]
    ar: GnarkG1Json,
    #[serde(rename = "Bs")]
    bs: GnarkG2Json,
    #[serde(rename = "Krs")]
    krs: GnarkG1Json,
    #[serde(rename = "Commitments", default)]
    commitments: Vec<Value>,
}

#[derive(Debug, Clone, Deserialize)]
struct GnarkG1Json {
    #[serde(rename = "X")]
    x: Value,
    #[serde(rename = "Y")]
    y: Value,
}

#[derive(Debug, Clone, Deserialize)]
struct GnarkG2Json {
    #[serde(rename = "X")]
    x: GnarkFq2Json,
    #[serde(rename = "Y")]
    y: GnarkFq2Json,
}

#[derive(Debug, Clone, Deserialize)]
struct GnarkFq2Json {
    #[serde(rename = "A0")]
    a0: Value,
    #[serde(rename = "A1")]
    a1: Value,
}

#[derive(Debug, Clone, Copy)]
struct GnarkEncoding {
    curve: CurveKind,
    field_bytes: usize,
    flag_mask: u8,
    compressed_smallest: u8,
    compressed_largest: u8,
    compressed_infinity: u8,
    uncompressed: u8,
    uncompressed_infinity: Option<u8>,
}

impl GnarkEncoding {
    fn for_curve(curve: CurveKind) -> Self {
        match curve {
            CurveKind::Bn254 => Self {
                curve,
                field_bytes: 32,
                flag_mask: 0b11 << 6,
                compressed_smallest: 0b10 << 6,
                compressed_largest: 0b11 << 6,
                compressed_infinity: 0b01 << 6,
                uncompressed: 0,
                uncompressed_infinity: None,
            },
            CurveKind::Bls12_381 => Self {
                curve,
                field_bytes: 48,
                flag_mask: 0b111 << 5,
                compressed_smallest: 0b100 << 5,
                compressed_largest: 0b101 << 5,
                compressed_infinity: 0b110 << 5,
                uncompressed: 0,
                uncompressed_infinity: Some(0b010 << 5),
            },
        }
    }

    fn g1_compressed_size(self) -> usize {
        self.field_bytes
    }

    fn g1_uncompressed_size(self) -> usize {
        self.field_bytes * 2
    }

    fn g2_compressed_size(self) -> usize {
        self.field_bytes * 2
    }

    fn g2_uncompressed_size(self) -> usize {
        self.field_bytes * 4
    }

    fn is_compressed_flag(self, flag: u8) -> bool {
        flag == self.compressed_smallest
            || flag == self.compressed_largest
            || flag == self.compressed_infinity
    }

    fn is_uncompressed_flag(self, flag: u8) -> bool {
        flag == self.uncompressed || Some(flag) == self.uncompressed_infinity
    }

    fn ark_flag(self, flag: u8) -> Result<u8> {
        if flag == self.compressed_smallest {
            Ok(0)
        } else if flag == self.compressed_largest {
            Ok(0b10 << 6)
        } else if flag == self.compressed_infinity {
            Ok(0b01 << 6)
        } else {
            Err(Error::Serialization(format!(
                "unsupported gnark compressed point flag 0x{flag:02x}"
            )))
        }
    }
}

pub fn load_gnark_json_inputs(
    vk_path: &Path,
    proof_path: Option<&Path>,
    public_path: Option<&Path>,
    curve_hint: Option<&str>,
) -> Result<Groth16VerifierInputs> {
    let vk: GnarkVerificationKeyJson = read_json(vk_path)?;
    ensure_no_vk_commitments(&vk)?;
    let verifying_key = vk_from_json(vk)?;
    let proof = proof_path.map(load_gnark_json_proof).transpose()?;
    let public_inputs = load_optional_public_inputs(public_path)?;

    match curve_hint {
        Some(raw_curve) => Groth16VerifierInputs::from_parts(
            CurveKind::from_name(raw_curve)?,
            verifying_key,
            proof,
            public_inputs,
            SourceFormat::GnarkJson,
        ),
        None => infer_curve(verifying_key, proof, public_inputs, SourceFormat::GnarkJson),
    }
}

pub fn load_gnark_binary_inputs(
    vk_path: &Path,
    proof_path: Option<&Path>,
    public_path: Option<&Path>,
    curve_hint: &str,
) -> Result<Groth16VerifierInputs> {
    let curve = CurveKind::from_name(curve_hint)?;
    load_gnark_binary_inputs_for_curve(vk_path, proof_path, public_path, curve)
}

pub fn load_gnark_binary_inputs_auto(
    vk_path: &Path,
    proof_path: Option<&Path>,
    public_path: Option<&Path>,
) -> Result<Groth16VerifierInputs> {
    let mut matches = Vec::new();
    let mut errors = Vec::new();
    for curve in [CurveKind::Bn254, CurveKind::Bls12_381] {
        match load_gnark_binary_inputs_for_curve(vk_path, proof_path, public_path, curve) {
            Ok(inputs) => matches.push(inputs),
            Err(err) => errors.push(format!("{}: {err}", curve.canonical_name())),
        }
    }

    match matches.len() {
        1 => Ok(matches.remove(0)),
        0 => Err(Error::MissingInput(format!(
            "could not auto-detect gnark binary curve: {}",
            errors.join("; ")
        ))),
        _ => Err(Error::CurveMismatch(
            "gnark binary artifact is ambiguous between supported curves".to_string(),
        )),
    }
}

fn load_gnark_binary_inputs_for_curve(
    vk_path: &Path,
    proof_path: Option<&Path>,
    public_path: Option<&Path>,
    curve: CurveKind,
) -> Result<Groth16VerifierInputs> {
    let verifying_key = load_gnark_binary_vk(vk_path, curve)?;
    let proof = proof_path
        .map(|path| load_gnark_binary_proof(path, curve))
        .transpose()?;
    let public_inputs = load_optional_public_inputs(public_path)?;

    Groth16VerifierInputs::from_parts(
        curve,
        verifying_key,
        proof,
        public_inputs,
        SourceFormat::GnarkBin,
    )
}

fn infer_curve(
    verifying_key: Groth16VerificationKey,
    proof: Option<Groth16Proof>,
    public_inputs: Vec<String>,
    source_format: SourceFormat,
) -> Result<Groth16VerifierInputs> {
    let mut matches = Vec::new();
    let mut errors = Vec::new();

    for curve in [CurveKind::Bn254, CurveKind::Bls12_381] {
        let inputs = Groth16VerifierInputs::from_parts(
            curve,
            verifying_key.clone(),
            proof.clone(),
            public_inputs.clone(),
            source_format,
        );
        let inputs = match inputs {
            Ok(inputs) => inputs,
            Err(err) => {
                errors.push(format!("{}: {err}", curve.canonical_name()));
                continue;
            }
        };
        let adapter = create_adapter(curve.canonical_name())?;
        let validation = if inputs.has_test_vectors() {
            match adapter.local_verify(&inputs) {
                Ok(true) => Ok(()),
                Ok(false) => Err(Error::LocalProofVerificationFailed(
                    "local verification returned false".to_string(),
                )),
                Err(err) => Err(err),
            }
        } else {
            adapter
                .serialize_verifying_key(&inputs.verifying_key)
                .map(|_| ())
        };

        match validation {
            Ok(()) => matches.push(inputs),
            Err(err) => errors.push(format!("{}: {err}", curve.canonical_name())),
        }
    }

    match matches.len() {
        1 => Ok(matches.remove(0)),
        0 => Err(Error::MissingInput(format!(
            "could not infer gnark curve: {}",
            errors.join("; ")
        ))),
        _ => Err(Error::CurveMismatch(
            "gnark JSON artifact is valid for more than one supported curve".to_string(),
        )),
    }
}

fn load_gnark_json_proof(path: &Path) -> Result<Groth16Proof> {
    let proof: GnarkProofJson = read_json(path)?;
    if !proof.commitments.is_empty() {
        return Err(Error::UnsupportedProtocol(
            "gnark proof commitments are not supported by Sui groth16 verifier generation"
                .to_string(),
        ));
    }
    Ok(Groth16Proof {
        pi_a: g1_from_json(proof.ar, "Ar")?,
        pi_b: g2_from_json(proof.bs, "Bs")?,
        pi_c: g1_from_json(proof.krs, "Krs")?,
    })
}

fn vk_from_json(vk: GnarkVerificationKeyJson) -> Result<Groth16VerificationKey> {
    let ic = vk
        .g1
        .k
        .into_iter()
        .enumerate()
        .map(|(idx, point)| g1_from_json(point, &format!("G1.K[{idx}]")))
        .collect::<Result<Vec<_>>>()?;

    Ok(Groth16VerificationKey {
        n_public: ic.len().saturating_sub(1),
        vk_alpha_1: g1_from_json(vk.g1.alpha, "G1.Alpha")?,
        vk_beta_2: g2_from_json(vk.g2.beta, "G2.Beta")?,
        vk_gamma_2: g2_from_json(vk.g2.gamma, "G2.Gamma")?,
        vk_delta_2: g2_from_json(vk.g2.delta, "G2.Delta")?,
        ic,
    })
}

fn ensure_no_vk_commitments(vk: &GnarkVerificationKeyJson) -> Result<()> {
    if !is_empty_or_null(&vk.commitment_keys) || !vk.public_and_commitment_committed.is_empty() {
        return Err(Error::UnsupportedProtocol(
            "gnark commitment keys are not supported by Sui groth16 verifier generation"
                .to_string(),
        ));
    }
    Ok(())
}

fn is_empty_or_null(value: &Option<Value>) -> bool {
    match value {
        None | Some(Value::Null) => true,
        Some(Value::Array(items)) => items.is_empty(),
        _ => false,
    }
}

fn g1_from_json(point: GnarkG1Json, name: &str) -> Result<Groth16G1Point> {
    Ok(Groth16G1Point {
        x: decimal_from_value(&point.x, &format!("{name}.X"))?,
        y: decimal_from_value(&point.y, &format!("{name}.Y"))?,
        z: "1".to_string(),
    })
}

fn g2_from_json(point: GnarkG2Json, name: &str) -> Result<Groth16G2Point> {
    Ok(Groth16G2Point {
        x0: decimal_from_value(&point.x.a0, &format!("{name}.X.A0"))?,
        x1: decimal_from_value(&point.x.a1, &format!("{name}.X.A1"))?,
        y0: decimal_from_value(&point.y.a0, &format!("{name}.Y.A0"))?,
        y1: decimal_from_value(&point.y.a1, &format!("{name}.Y.A1"))?,
        z0: "1".to_string(),
        z1: "0".to_string(),
    })
}

fn load_gnark_binary_vk(path: &Path, curve: CurveKind) -> Result<Groth16VerificationKey> {
    let bytes = read_bytes(path)?;
    let encoding = GnarkEncoding::for_curve(curve);
    let mut reader = GnarkBinaryReader::new(&bytes, path);

    let alpha = reader.read_g1(encoding, "G1.Alpha")?;
    let _beta_g1 = reader.read_g1(encoding, "G1.Beta")?;
    let beta = reader.read_g2(encoding, "G2.Beta")?;
    let gamma = reader.read_g2(encoding, "G2.Gamma")?;
    let _delta_g1 = reader.read_g1(encoding, "G1.Delta")?;
    let delta = reader.read_g2(encoding, "G2.Delta")?;
    let ic_len = reader.read_u32("G1.K length")? as usize;
    let mut ic = Vec::with_capacity(ic_len);
    for idx in 0..ic_len {
        ic.push(reader.read_g1(encoding, &format!("G1.K[{idx}]"))?);
    }
    let public_and_commitment_committed =
        reader.read_u64_slice_slice("PublicAndCommitmentCommitted")?;
    let commitment_count = reader.read_u32("CommitmentKeys length")?;
    if !public_and_commitment_committed.is_empty() || commitment_count != 0 {
        return Err(Error::UnsupportedProtocol(
            "gnark commitment keys are not supported by Sui groth16 verifier generation"
                .to_string(),
        ));
    }
    reader.finish()?;

    Ok(Groth16VerificationKey {
        n_public: ic.len().saturating_sub(1),
        vk_alpha_1: alpha,
        vk_beta_2: beta,
        vk_gamma_2: gamma,
        vk_delta_2: delta,
        ic,
    })
}

fn load_gnark_binary_proof(path: &Path, curve: CurveKind) -> Result<Groth16Proof> {
    let bytes = read_bytes(path)?;
    let encoding = GnarkEncoding::for_curve(curve);
    let mut reader = GnarkBinaryReader::new(&bytes, path);

    let ar = reader.read_g1(encoding, "Ar")?;
    let bs = reader.read_g2(encoding, "Bs")?;
    let krs = reader.read_g1(encoding, "Krs")?;
    let commitment_count = reader.read_u32("Commitments length")?;
    if commitment_count != 0 {
        return Err(Error::UnsupportedProtocol(
            "gnark proof commitments are not supported by Sui groth16 verifier generation"
                .to_string(),
        ));
    }
    let _commitment_pok = reader.read_g1(encoding, "CommitmentPok")?;
    reader.finish()?;

    Ok(Groth16Proof {
        pi_a: ar,
        pi_b: bs,
        pi_c: krs,
    })
}

struct GnarkBinaryReader<'a> {
    bytes: &'a [u8],
    offset: usize,
    path: &'a Path,
}

impl<'a> GnarkBinaryReader<'a> {
    fn new(bytes: &'a [u8], path: &'a Path) -> Self {
        Self {
            bytes,
            offset: 0,
            path,
        }
    }

    fn read_g1(&mut self, encoding: GnarkEncoding, field: &str) -> Result<Groth16G1Point> {
        let first = self.peek_byte(field)?;
        let flag = first & encoding.flag_mask;
        let compressed = if encoding.is_compressed_flag(flag) {
            true
        } else if encoding.is_uncompressed_flag(flag) {
            false
        } else {
            return Err(Error::Serialization(format!(
                "{field} has unsupported gnark point flag 0x{flag:02x}"
            )));
        };
        let len = if compressed {
            encoding.g1_compressed_size()
        } else {
            encoding.g1_uncompressed_size()
        };
        let bytes = self.read_exact(len, field)?;
        decode_gnark_g1(bytes, encoding, compressed, field)
    }

    fn read_g2(&mut self, encoding: GnarkEncoding, field: &str) -> Result<Groth16G2Point> {
        let first = self.peek_byte(field)?;
        let flag = first & encoding.flag_mask;
        let compressed = if encoding.is_compressed_flag(flag) {
            true
        } else if encoding.is_uncompressed_flag(flag) {
            false
        } else {
            return Err(Error::Serialization(format!(
                "{field} has unsupported gnark point flag 0x{flag:02x}"
            )));
        };
        let len = if compressed {
            encoding.g2_compressed_size()
        } else {
            encoding.g2_uncompressed_size()
        };
        let bytes = self.read_exact(len, field)?;
        decode_gnark_g2(bytes, encoding, compressed, field)
    }

    fn read_u32(&mut self, field: &str) -> Result<u32> {
        let bytes = self.read_exact(4, field)?;
        Ok(u32::from_be_bytes(bytes.try_into().unwrap()))
    }

    fn read_u64(&mut self, field: &str) -> Result<u64> {
        let bytes = self.read_exact(8, field)?;
        Ok(u64::from_be_bytes(bytes.try_into().unwrap()))
    }

    fn read_u64_slice_slice(&mut self, field: &str) -> Result<Vec<Vec<u64>>> {
        let outer_len = self.read_u32(&format!("{field} length"))? as usize;
        let mut outer = Vec::with_capacity(outer_len);
        for outer_idx in 0..outer_len {
            let inner_len = self.read_u32(&format!("{field}[{outer_idx}] length"))? as usize;
            let mut inner = Vec::with_capacity(inner_len);
            for inner_idx in 0..inner_len {
                inner.push(self.read_u64(&format!("{field}[{outer_idx}][{inner_idx}]"))?);
            }
            outer.push(inner);
        }
        Ok(outer)
    }

    fn finish(&self) -> Result<()> {
        if self.offset == self.bytes.len() {
            return Ok(());
        }
        Err(Error::Serialization(format!(
            "{} has {} trailing bytes after gnark artifact",
            self.path.display(),
            self.bytes.len() - self.offset
        )))
    }

    fn peek_byte(&self, field: &str) -> Result<u8> {
        self.bytes.get(self.offset).copied().ok_or_else(|| {
            Error::Serialization(format!(
                "{} ended before reading {field}",
                self.path.display()
            ))
        })
    }

    fn read_exact(&mut self, len: usize, field: &str) -> Result<&'a [u8]> {
        let end = self.offset.checked_add(len).ok_or_else(|| {
            Error::Serialization(format!(
                "overflow while reading {field} from gnark artifact"
            ))
        })?;
        if end > self.bytes.len() {
            return Err(Error::Serialization(format!(
                "{} ended while reading {field}: need {len} bytes at offset {}, remaining {}",
                self.path.display(),
                self.offset,
                self.bytes.len().saturating_sub(self.offset)
            )));
        }
        let slice = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(slice)
    }
}

fn decode_gnark_g1(
    bytes: &[u8],
    encoding: GnarkEncoding,
    compressed: bool,
    field: &str,
) -> Result<Groth16G1Point> {
    match (encoding.curve, compressed) {
        (CurveKind::Bn254, true) => {
            let ark = gnark_compressed_g1_to_ark(bytes, encoding)?;
            let mut reader = ark.as_slice();
            let point = Bn254G1Affine::deserialize_compressed(&mut reader).map_err(|e| {
                Error::Serialization(format!("failed to deserialize BN254 {field}: {e:?}"))
            })?;
            Ok(point_from_bn254_g1(&point))
        }
        (CurveKind::Bn254, false) => {
            let ark = gnark_uncompressed_g1_to_ark(bytes, encoding);
            let mut reader = ark.as_slice();
            let point = Bn254G1Affine::deserialize_uncompressed(&mut reader).map_err(|e| {
                Error::Serialization(format!("failed to deserialize BN254 {field}: {e:?}"))
            })?;
            Ok(point_from_bn254_g1(&point))
        }
        (CurveKind::Bls12_381, true) => {
            let ark = gnark_compressed_g1_to_ark(bytes, encoding)?;
            let mut reader = ark.as_slice();
            let point = BlsG1Affine::deserialize_compressed(&mut reader).map_err(|e| {
                Error::Serialization(format!("failed to deserialize BLS12-381 {field}: {e:?}"))
            })?;
            Ok(point_from_bls_g1(&point))
        }
        (CurveKind::Bls12_381, false) => {
            let ark = gnark_uncompressed_g1_to_ark(bytes, encoding);
            let mut reader = ark.as_slice();
            let point = BlsG1Affine::deserialize_uncompressed(&mut reader).map_err(|e| {
                Error::Serialization(format!("failed to deserialize BLS12-381 {field}: {e:?}"))
            })?;
            Ok(point_from_bls_g1(&point))
        }
    }
}

fn decode_gnark_g2(
    bytes: &[u8],
    encoding: GnarkEncoding,
    compressed: bool,
    field: &str,
) -> Result<Groth16G2Point> {
    match (encoding.curve, compressed) {
        (CurveKind::Bn254, true) => {
            let ark = gnark_compressed_g2_to_ark(bytes, encoding)?;
            let mut reader = ark.as_slice();
            let point = Bn254G2Affine::deserialize_compressed(&mut reader).map_err(|e| {
                Error::Serialization(format!("failed to deserialize BN254 {field}: {e:?}"))
            })?;
            Ok(point_from_bn254_g2(&point))
        }
        (CurveKind::Bn254, false) => {
            let ark = gnark_uncompressed_g2_to_ark(bytes, encoding);
            let mut reader = ark.as_slice();
            let point = Bn254G2Affine::deserialize_uncompressed(&mut reader).map_err(|e| {
                Error::Serialization(format!("failed to deserialize BN254 {field}: {e:?}"))
            })?;
            Ok(point_from_bn254_g2(&point))
        }
        (CurveKind::Bls12_381, true) => {
            let ark = gnark_compressed_g2_to_ark(bytes, encoding)?;
            let mut reader = ark.as_slice();
            let point = BlsG2Affine::deserialize_compressed(&mut reader).map_err(|e| {
                Error::Serialization(format!("failed to deserialize BLS12-381 {field}: {e:?}"))
            })?;
            Ok(point_from_bls_g2(&point))
        }
        (CurveKind::Bls12_381, false) => {
            let ark = gnark_uncompressed_g2_to_ark(bytes, encoding);
            let mut reader = ark.as_slice();
            let point = BlsG2Affine::deserialize_uncompressed(&mut reader).map_err(|e| {
                Error::Serialization(format!("failed to deserialize BLS12-381 {field}: {e:?}"))
            })?;
            Ok(point_from_bls_g2(&point))
        }
    }
}

fn gnark_compressed_g1_to_ark(bytes: &[u8], encoding: GnarkEncoding) -> Result<Vec<u8>> {
    if encoding.curve == CurveKind::Bls12_381 {
        return Ok(bytes.to_vec());
    }
    let flag = bytes[0] & encoding.flag_mask;
    let mut cleaned = bytes.to_vec();
    cleaned[0] &= !encoding.flag_mask;
    let mut out = reverse_field_bytes(&cleaned);
    let last = out.len() - 1;
    out[last] |= encoding.ark_flag(flag)?;
    Ok(out)
}

fn gnark_uncompressed_g1_to_ark(bytes: &[u8], encoding: GnarkEncoding) -> Vec<u8> {
    if encoding.curve == CurveKind::Bls12_381 {
        return bytes.to_vec();
    }
    let coordinate_size = encoding.field_bytes;
    let mut x = bytes[..coordinate_size].to_vec();
    x[0] &= !encoding.flag_mask;
    let y = &bytes[coordinate_size..coordinate_size * 2];

    let mut out = Vec::with_capacity(bytes.len());
    out.extend(reverse_field_bytes(&x));
    out.extend(reverse_field_bytes(y));
    out
}

fn gnark_compressed_g2_to_ark(bytes: &[u8], encoding: GnarkEncoding) -> Result<Vec<u8>> {
    if encoding.curve == CurveKind::Bls12_381 {
        return Ok(bytes.to_vec());
    }
    let coordinate_size = encoding.field_bytes;
    let flag = bytes[0] & encoding.flag_mask;
    let mut x1 = bytes[..coordinate_size].to_vec();
    x1[0] &= !encoding.flag_mask;
    let x0 = &bytes[coordinate_size..coordinate_size * 2];

    let mut out = Vec::with_capacity(bytes.len());
    out.extend(reverse_field_bytes(x0));
    out.extend(reverse_field_bytes(&x1));
    let last = out.len() - 1;
    out[last] |= encoding.ark_flag(flag)?;

    Ok(out)
}

fn gnark_uncompressed_g2_to_ark(bytes: &[u8], encoding: GnarkEncoding) -> Vec<u8> {
    if encoding.curve == CurveKind::Bls12_381 {
        return bytes.to_vec();
    }
    let coordinate_size = encoding.field_bytes;
    let mut x1 = bytes[..coordinate_size].to_vec();
    x1[0] &= !encoding.flag_mask;
    let x0 = &bytes[coordinate_size..coordinate_size * 2];
    let y1 = &bytes[coordinate_size * 2..coordinate_size * 3];
    let y0 = &bytes[coordinate_size * 3..coordinate_size * 4];

    let mut out = Vec::with_capacity(bytes.len());
    out.extend(reverse_field_bytes(x0));
    out.extend(reverse_field_bytes(&x1));
    out.extend(reverse_field_bytes(y0));
    out.extend(reverse_field_bytes(y1));
    out
}

fn reverse_field_bytes(bytes: &[u8]) -> Vec<u8> {
    bytes.iter().rev().copied().collect()
}

fn point_from_bn254_g1(point: &Bn254G1Affine) -> Groth16G1Point {
    Groth16G1Point {
        x: point.x.to_string(),
        y: point.y.to_string(),
        z: "1".to_string(),
    }
}

fn point_from_bn254_g2(point: &Bn254G2Affine) -> Groth16G2Point {
    Groth16G2Point {
        x0: point.x.c0.to_string(),
        x1: point.x.c1.to_string(),
        y0: point.y.c0.to_string(),
        y1: point.y.c1.to_string(),
        z0: "1".to_string(),
        z1: "0".to_string(),
    }
}

fn point_from_bls_g1(point: &BlsG1Affine) -> Groth16G1Point {
    Groth16G1Point {
        x: point.x.to_string(),
        y: point.y.to_string(),
        z: "1".to_string(),
    }
}

fn point_from_bls_g2(point: &BlsG2Affine) -> Groth16G2Point {
    Groth16G2Point {
        x0: point.x.c0.to_string(),
        x1: point.x.c1.to_string(),
        y0: point.y.c0.to_string(),
        y1: point.y.c1.to_string(),
        z0: "1".to_string(),
        z1: "0".to_string(),
    }
}

fn load_optional_public_inputs(path: Option<&Path>) -> Result<Vec<String>> {
    match path {
        Some(path) => parse_public_inputs_value(&read_json_value(path)?, "public"),
        None => Ok(Vec::new()),
    }
}

fn parse_public_inputs_value(value: &Value, field: &str) -> Result<Vec<String>> {
    match value {
        Value::Array(values) => values
            .iter()
            .enumerate()
            .map(|(idx, value)| decimal_from_value(value, &format!("{field}[{idx}]")))
            .collect(),
        Value::Object(map) => {
            for key in ["public_inputs", "public", "publicSignals"] {
                if let Some(inner) = map.get(key) {
                    return parse_public_inputs_value(inner, key);
                }
            }
            if map.len() == 1 {
                let (key, value) = map.iter().next().unwrap();
                return decimal_from_value(value, key).map(|value| vec![value]);
            }
            Err(Error::MissingInput(
                "gnark public input object with multiple fields must use an ordered public_inputs array"
                    .to_string(),
            ))
        }
        _ => decimal_from_value(value, field).map(|value| vec![value]),
    }
}

fn decimal_from_value(value: &Value, field: &str) -> Result<String> {
    match value {
        Value::String(raw) => {
            parse_decimal(raw, field)?;
            Ok(raw.clone())
        }
        Value::Number(num) => {
            let decimal = num.to_string();
            parse_decimal(&decimal, field)?;
            Ok(decimal)
        }
        _ => Err(Error::DecimalParse(format!(
            "{field} must be a decimal string or number"
        ))),
    }
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let content = fs::read_to_string(path).map_err(|e| Error::Io {
        source: e,
        context: format!("failed to read file {}", path.display()),
    })?;
    serde_json::from_str::<T>(&content).map_err(|e| Error::JsonParse {
        source: e,
        context: format!("invalid gnark json in file {}", path.display()),
    })
}

fn read_json_value(path: &Path) -> Result<Value> {
    read_json(path)
}

fn read_bytes(path: &Path) -> Result<Vec<u8>> {
    fs::read(path).map_err(|e| Error::Io {
        source: e,
        context: format!("failed to read file {}", path.display()),
    })
}
