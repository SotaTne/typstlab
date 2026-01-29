/**
 * @file Typstlabtypst grammar for tree-sitter
 * @author sotatne
 * @license MIT
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

export default grammar({
  name: "typstlabtypst",
  rules: {
    source_file: $ => repeat("hello")
  }
});
