package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/test"
)

func TestDefaultOutDirIsExampleLocal(t *testing.T) {
	got := filepath.ToSlash(defaultOutDir())
	wantSuffix := "examples/gnark-native/mimc/artifacts"
	if !strings.HasSuffix(got, wantSuffix) {
		t.Fatalf("defaultOutDir() = %q, want suffix %q", got, wantSuffix)
	}
}

func TestCircuitAcceptsOnlyMatchingMiMCHash(t *testing.T) {
	for _, curve := range []struct {
		name string
		id   ecc.ID
	}{
		{name: "bn254", id: ecc.BN254},
		{name: "bls12381", id: ecc.BLS12_381},
	} {
		t.Run(curve.name, func(t *testing.T) {
			assignment, err := ValidAssignment(curve.id)
			if err != nil {
				t.Fatal(err)
			}

			assert := test.NewAssert(t)
			assert.CheckCircuit(
				&Circuit{},
				test.WithValidAssignment(assignment),
				test.WithInvalidAssignment(InvalidAssignment()),
				test.WithCurves(curve.id),
			)
		})
	}
}

func TestValidAssignmentUsesCurveSpecificMiMCHash(t *testing.T) {
	bn254Assignment, err := ValidAssignment(ecc.BN254)
	if err != nil {
		t.Fatal(err)
	}
	bls12381Assignment, err := ValidAssignment(ecc.BLS12_381)
	if err != nil {
		t.Fatal(err)
	}

	if bn254Assignment.Hash == "42" || bls12381Assignment.Hash == "42" {
		t.Fatal("valid assignment used invalid hash sentinel")
	}
	if bn254Assignment.Hash == bls12381Assignment.Hash {
		t.Fatalf("expected curve-specific MiMC hashes, got %v", bn254Assignment.Hash)
	}
}

func TestWriteMiMCArtifacts(t *testing.T) {
	outDir := filepath.Join(t.TempDir(), "bn254")
	if err := writeArtifacts(outDir, ecc.BN254); err != nil {
		t.Fatalf("writeArtifacts() error = %v", err)
	}

	for _, name := range []string{
		"verification_key_gnark.json",
		"proof_gnark.json",
		"verification_key.bin",
		"proof.bin",
		"public.json",
	} {
		if _, err := os.Stat(filepath.Join(outDir, name)); err != nil {
			t.Fatalf("expected artifact %s: %v", name, err)
		}
	}

	rawPublic, err := os.ReadFile(filepath.Join(outDir, "public.json"))
	if err != nil {
		t.Fatal(err)
	}
	var public []string
	if err := json.Unmarshal(rawPublic, &public); err != nil {
		t.Fatal(err)
	}
	if len(public) != 1 || public[0] == "" || public[0] == "42" {
		t.Fatalf("public.json = %#v, want one computed MiMC digest", public)
	}
}
