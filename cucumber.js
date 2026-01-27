// cucumber.js
let common = [
  'features/**/*.feature',                // Specify our feature files
  '--require features/**/*.ts',           // Load step definitions
  '-f @cucumber/pretty-formatter',
//  '-t @new',
  '--no-strict',
  '--force-exit',
].join(' ');

module.exports = {
  default: common
};
