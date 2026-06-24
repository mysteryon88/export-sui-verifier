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
	wantSuffix := "examples/gnark-native/cubic/artifacts"
	if !strings.HasSuffix(got, wantSuffix) {
		t.Fatalf("defaultOutDir() = %q, want suffix %q", got, wantSuffix)
	}
}

func TestCircuitAcceptsOnlyCubicRelation(t *testing.T) {
	assert := test.NewAssert(t)
	assert.CheckCircuit(
		&Circuit{},
		test.WithValidAssignment(ValidAssignment()),
		test.WithInvalidAssignment(InvalidAssignment()),
	)
}

func TestWriteCubicArtifacts(t *testing.T) {
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
	if len(public) != 1 || public[0] != "35" {
		t.Fatalf("public.json = %#v, want [35]", public)
	}
}
