module.exports = function addVersionIgnoresKeyword(ajv) {
  ajv.addKeyword({
    keyword: "version_ignores",
    schemaType: "array",
    validate: () => true,
    errors: false,
  });
};
