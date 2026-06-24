package main

import (
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"runtime"

	"export-sui-verifier/examples/gnark-native/internal/artifacts"
	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/frontend"
)

type Circuit struct {
	X frontend.Variable
	Y frontend.Variable `gnark:",public"`
}

func (circuit *Circuit) Define(api frontend.API) error {
	x3 := api.Mul(circuit.X, circuit.X, circuit.X)
	api.AssertIsEqual(circuit.Y, api.Add(x3, circuit.X, 5))
	return nil
}

func ValidAssignment() *Circuit {
	return &Circuit{X: 3, Y: 35}
}

func InvalidAssignment() *Circuit {
	return &Circuit{X: 3, Y: 34}
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
	return artifacts.WriteGroth16Artifacts(outDir, curve, &Circuit{}, ValidAssignment())
}
