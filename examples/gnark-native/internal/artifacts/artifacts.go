package artifacts

import (
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path/filepath"

	"github.com/consensys/gnark-crypto/ecc"
	bls12381fr "github.com/consensys/gnark-crypto/ecc/bls12-381/fr"
	bn254fr "github.com/consensys/gnark-crypto/ecc/bn254/fr"
	"github.com/consensys/gnark/backend/groth16"
	"github.com/consensys/gnark/backend/witness"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/frontend/cs/r1cs"
)

type Curve struct {
	Slug string
	ID   ecc.ID
}

func SupportedCurves() []Curve {
	return []Curve{
		{Slug: "bn254", ID: ecc.BN254},
		{Slug: "bls12381", ID: ecc.BLS12_381},
	}
}

func SelectCurves(raw string) ([]Curve, error) {
	switch raw {
	case "", "all":
		return SupportedCurves(), nil
	case "bn254":
		return []Curve{{Slug: "bn254", ID: ecc.BN254}}, nil
	case "bls12381", "bls12_381", "bls12-381":
		return []Curve{{Slug: "bls12381", ID: ecc.BLS12_381}}, nil
	default:
		return nil, fmt.Errorf("unsupported curve %q; use all, bn254, or bls12381", raw)
	}
}

func WriteGroth16Artifacts(outDir string, curve ecc.ID, circuit, assignment frontend.Circuit) error {
	ccs, err := frontend.Compile(curve.ScalarField(), r1cs.NewBuilder, circuit)
	if err != nil {
		return fmt.Errorf("compile circuit: %w", err)
	}

	pk, vk, err := groth16.Setup(ccs)
	if err != nil {
		return fmt.Errorf("setup Groth16: %w", err)
	}

	fullWitness, err := frontend.NewWitness(assignment, curve.ScalarField())
	if err != nil {
		return fmt.Errorf("build full witness: %w", err)
	}

	publicWitness, err := fullWitness.Public()
	if err != nil {
		return fmt.Errorf("extract public witness: %w", err)
	}

	proof, err := groth16.Prove(ccs, pk, fullWitness)
	if err != nil {
		return fmt.Errorf("prove: %w", err)
	}

	if err := groth16.Verify(proof, vk, publicWitness); err != nil {
		return fmt.Errorf("verify proof: %w", err)
	}

	publicSignals, err := PublicSignals(publicWitness)
	if err != nil {
		return err
	}

	if err := os.MkdirAll(outDir, 0o755); err != nil {
		return fmt.Errorf("create artifact dir %s: %w", outDir, err)
	}
	if err := writeJSON(filepath.Join(outDir, "verification_key_gnark.json"), vk); err != nil {
		return err
	}
	if err := writeJSON(filepath.Join(outDir, "proof_gnark.json"), proof); err != nil {
		return err
	}
	if err := writeJSON(filepath.Join(outDir, "public.json"), publicSignals); err != nil {
		return err
	}
	if err := writeBinary(filepath.Join(outDir, "verification_key.bin"), vk); err != nil {
		return err
	}
	if err := writeBinary(filepath.Join(outDir, "proof.bin"), proof); err != nil {
		return err
	}

	return nil
}

func PublicSignals(w witness.Witness) ([]string, error) {
	switch vector := w.Vector().(type) {
	case bn254fr.Vector:
		return bn254PublicSignals(vector), nil
	case *bn254fr.Vector:
		return bn254PublicSignals(*vector), nil
	case bls12381fr.Vector:
		return bls12381PublicSignals(vector), nil
	case *bls12381fr.Vector:
		return bls12381PublicSignals(*vector), nil
	default:
		return nil, fmt.Errorf("unsupported public witness vector type %T", w.Vector())
	}
}

func bn254PublicSignals(vector bn254fr.Vector) []string {
	signals := make([]string, len(vector))
	for i := range vector {
		signals[i] = vector[i].String()
	}
	return signals
}

func bls12381PublicSignals(vector bls12381fr.Vector) []string {
	signals := make([]string, len(vector))
	for i := range vector {
		signals[i] = vector[i].String()
	}
	return signals
}

func writeJSON(path string, value any) error {
	file, err := os.Create(path)
	if err != nil {
		return fmt.Errorf("create %s: %w", path, err)
	}
	defer file.Close()

	encoder := json.NewEncoder(file)
	encoder.SetIndent("", "  ")
	if err := encoder.Encode(value); err != nil {
		return fmt.Errorf("write %s: %w", path, err)
	}
	return nil
}

type writerTo interface {
	WriteTo(io.Writer) (int64, error)
}

func writeBinary(path string, value writerTo) error {
	file, err := os.Create(path)
	if err != nil {
		return fmt.Errorf("create %s: %w", path, err)
	}
	defer file.Close()

	if _, err := value.WriteTo(file); err != nil {
		return fmt.Errorf("write %s: %w", path, err)
	}
	return nil
}
