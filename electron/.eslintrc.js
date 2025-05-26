/** @type {import("eslint").Linter.Config} */
module.exports = {
  root: true,
  extends: ["./eslint-library.js"],
  parserOptions: {
    project: true,
  },
};
