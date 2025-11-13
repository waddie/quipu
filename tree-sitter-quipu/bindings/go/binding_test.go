package tree_sitter_quipu_test

import (
	"testing"

	tree_sitter "github.com/tree-sitter/go-tree-sitter"
	tree_sitter_quipu "github.com/waddie/quipu/bindings/go"
)

func TestCanLoadGrammar(t *testing.T) {
	language := tree_sitter.NewLanguage(tree_sitter_quipu.Language())
	if language == nil {
		t.Errorf("Error loading quipu grammar")
	}
}
