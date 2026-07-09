/* eslint-disable no-undef */

const devCerts = require("office-addin-dev-certs");
const CopyWebpackPlugin = require("copy-webpack-plugin");
const CustomFunctionsMetadataPlugin = require("custom-functions-metadata-plugin");
const HtmlWebpackPlugin = require("html-webpack-plugin");
const path = require("path");
const webpack = require("webpack");

const urlDev = "https://localhost:3000/";
const urlProd = "https://www.contoso.com/"; // CHANGE THIS TO YOUR PRODUCTION DEPLOYMENT LOCATION

// teeline-wasm's jco-transpiled bindings target WASI Preview 2, which needs
// @bytecodealliance/preview2-shim to polyfill host imports (clocks, random —
// solve() calls Instant::now(), so this genuinely executes). The shim ships
// separate node/ and browser/ variants selected via package.json "exports"
// conditions; webpack picks the "node" condition here (this only exists as a
// transitive dependency of teeline-wasm/js-bindings, not a direct dependency,
// so its own package.json browser field/conditions aren't controlling the
// resolution), which pulls in "node:fs/promises" and fails to bundle for a
// browser target. Alias each subpath straight to the browser/ file to force
// the correct variant — mirrors teeline-web's force-preview2-shim-browser
// vite plugin (teeline-web/vite.config.ts), just as a webpack alias instead.
const preview2ShimBrowserDir = path.resolve(
  __dirname,
  "../teeline-wasm/js-bindings/node_modules/@bytecodealliance/preview2-shim/lib/browser"
);
const preview2ShimAliases = Object.fromEntries(
  ["cli", "clocks", "io", "random"].map((name) => [
    `@bytecodealliance/preview2-shim/${name}`,
    path.join(preview2ShimBrowserDir, `${name}.js`),
  ])
);

/* global require, module, process, __dirname */

async function getHttpsOptions() {
  const httpsOptions = await devCerts.getHttpsServerOptions();
  return { ca: httpsOptions.ca, key: httpsOptions.key, cert: httpsOptions.cert };
}

module.exports = async (env, options) => {
  const dev = options.mode === "development";
  const config = {
    devtool: "source-map",
    entry: {
      polyfill: ["core-js/stable", "regenerator-runtime/runtime"],
      taskpane: ["./src/taskpane/taskpane.ts", "./src/taskpane/taskpane.html"],
      commands: "./src/commands/commands.ts",
      functions: "./src/functions/functions.ts",
    },
    output: {
      clean: true,
    },
    resolve: {
      extensions: [".ts", ".html", ".js"],
      alias: preview2ShimAliases,
    },
    module: {
      rules: [
        {
          test: /\.ts$/,
          exclude: /node_modules/,
          use: {
            loader: "babel-loader"
          },
        },
        {
          test: /\.html$/,
          exclude: /node_modules/,
          use: "html-loader",
        },
        {
          test: /\.(png|jpg|jpeg|gif|ico)$/,
          type: "asset/resource",
          generator: {
            filename: "assets/[name][ext][query]",
          },
        },
      ],
    },
    plugins: [
      // jco's generated glue code has a dynamic `import('node:fs/promises')`
      // gated behind a runtime `isNode` check (teeline_wasm.js: fetchCompile).
      // That branch never runs in a browser, but webpack still tries to
      // statically resolve the "node:" scheme at build time and fails since
      // no plugin handles it. Ignore it — the browser fetch() fallback in
      // the same function is what actually runs here.
      new webpack.IgnorePlugin({ resourceRegExp: /^node:fs\/promises$/ }),
      new CustomFunctionsMetadataPlugin({
        output: "functions.json",
        input: "./src/functions/functions.ts",
      }),
      new HtmlWebpackPlugin({
        filename: "taskpane.html",
        template: "./src/taskpane/taskpane.html",
        chunks: ["polyfill", "taskpane", "functions", "commands"],
      }),
      new CopyWebpackPlugin({
        patterns: [
          {
            from: "assets/*",
            to: "assets/[name][ext][query]",
          },
          {
            from: "manifest*.xml",
            to: "[name]" + "[ext]",
            transform(content) {
              if (dev) {
                return content;
              } else {
                return content.toString().replace(urlDev, urlProd);
              }
            },
          },
        ],
      }),
    ],
    devServer: {
      headers: {
        "Access-Control-Allow-Origin": "*",
      },
      server: {
        type: "https",
        options: env.WEBPACK_BUILD || options.https !== undefined ? options.https : await getHttpsOptions(),
      },
      port: process.env.npm_package_config_dev_server_port || 3000,
    },
  };

  return config;
};
