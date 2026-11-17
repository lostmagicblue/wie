import webpack from "webpack";
import { merge } from "webpack-merge";
import { sentryWebpackPlugin } from "@sentry/webpack-plugin";

// @ts-ignore: allowImportingTsExtensions
import commonConfig from "./webpack.config.common.ts";

const config: webpack.Configuration = merge(commonConfig, {
  mode: "production",
  // Self-hosted builds do not need to upload source maps to upstream Sentry.
  // Keep upstream upload support when an explicit token is supplied.
  devtool: process.env.SENTRY_AUTH_TOKEN ? "hidden-source-map" : false,
  plugins: process.env.SENTRY_AUTH_TOKEN ? [
    sentryWebpackPlugin({
      // Fail the build on upload errors, which the plugin otherwise only logs.
      errorHandler(error) { throw error; },
      sourcemaps: {
        filesToDeleteAfterUpload: ["wie-web/dist/**/*.map"],
      },
    }),
  ] : [],
});

export default config;
