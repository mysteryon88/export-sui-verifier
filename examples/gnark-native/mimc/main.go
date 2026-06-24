package main

import (
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"runtime"

	"export-sui-verifier/examples/gnark-native/internal/artifacts"
	"github.com/consensys/gnark-crypto/ecc"
	bls12381fr "github.com/consensys/gnark-crypto/ecc/bls12-381/fr"
	bls12381mimc "github.com/consensys/gnark-crypto/ecc/bls12-381/fr/mimc"
	bn254fr "github.com/consensys/gnark-crypto/ecc/bn254/fr"
	bn254mimc "github.com/consensys/gnark-crypto/ecc/bn254/fr/mimc"
	"github.com/consensys/gnark/frontend"
	gnarkmimc "github.com/consensys/gnark/std/hash/mimc"
)

const PreImage = "500304"

type Circuit struct {
	PreImage frontend.Variable
	Hash     frontend.Variable `gnark:",public"`
}

func (circuit *Circuit) Define(api frontend.API) error {
	h, err := gnarkmimc.NewMiMC(api)
	if err != nil {
		return err
	}
	h.Write(circuit.PreImage)
	api.AssertIsEqual(circuit.Hash, h.Sum())
	return nil
}

func ValidAssignment(curve ecc.ID) (*Circuit, error) {
	digest, err := nativeMiMCDigest(curve, PreImage)
	if err != nil {
		return nil, err
	}
	return &Circuit{PreImage: PreImage, Hash: digest}, nil
}

func InvalidAssignment() *Circuit {
	return &Circuit{PreImage: PreImage, Hash: 42}
}

func main() {
	outDir := flag.String("out", defaultOutDir(), "directory where native gnark artifacts are written")
	curveFlag := flag.String("curve", "all", "curve to generate: all, bn254, or bls12381")
	flag.Parse()

	curves, err := artifacts.SelectCurves(*curveFlag)
	if err != nil {
		fmt.Fprintf(os.Stderr, "%v\n", err)
		os.Exit(1)
	}

	for _, target := range curves {
		curveOutDir := filepath.Join(*outDir, target.Slug)
		if err := writeArtifacts(curveOutDir, target.ID); err != nil {
			fmt.Fprintf(os.Stderr, "%s: %v\n", target.Slug, err)
			os.Exit(1)
		}
		fmt.Printf("wrote %s artifacts to %s\n", target.Slug, curveOutDir)
	}
}

func defaultOutDir() string {
	_, file, _, ok := runtime.Caller(0)
	if !ok {
		return "artifacts"
	}
	return filepath.Join(filepath.Dir(file), "artifacts")
}

func writeArtifacts(outDir string, curve ecc.ID) error {
	assignment, err := ValidAssignment(curve)
	if err != nil {
		return err
	}
	return artifacts.WriteGroth16Artifacts(outDir, curve, &Circuit{}, assignment)
}

func nativeMiMCDigest(curve ecc.ID, input string) (string, error) {
	switch curve {
	case ecc.BN254:
		var x bn254fr.Element
		if _, err := x.SetString(input); err != nil {
			return "", fmt.Errorf("parse BN254 field element: %w", err)
		}
		h := bn254mimc.NewMiMC()
		b := x.Bytes()
		if _, err := h.Write(b[:]); err != nil {
			return "", fmt.Errorf("hash BN254 MiMC input: %w", err)
		}
		x.SetBytes(h.Sum(nil))
		return x.String(), nil
	case ecc.BLS12_381:
		var x bls12381fr.Element
		if _, err := x.SetString(input); err != nil {
			return "", fmt.Errorf("parse BLS12-381 field element: %w", err)
		}
		h := bls12381mimc.NewMiMC()
		b := x.Bytes()
		if _, err := h.Write(b[:]); err != nil {
			return "", fmt.Errorf("hash BLS12-381 MiMC input: %w", err)
		}
		x.SetBytes(h.Sum(nil))
		return x.String(), nil
	default:
		return "", fmt.Errorf("unsupported MiMC curve %v", curve)
	}
}
