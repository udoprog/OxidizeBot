const HtmlWebPackPlugin = require("html-webpack-plugin");
const FaviconsWebpackPlugin = require('favicons-webpack-plugin')
const path = require('path');

const htmlPlugin = new HtmlWebPackPlugin({
  template: "./src/index.html",
  filename: "./index.html"
});

const faviconPlugin = new FaviconsWebpackPlugin("../../bot/res/icon.png");

module.exports = function(_, argv) {
  let production = argv.mode === "production";

  return {
    output: {
      path: path.join(__dirname, 'dist'),
      filename: '[name].[chunkhash].js',
      chunkFilename: '[name].[chunkhash].js'
    },
    optimization: {
      minimize: production
    },
    module: {
      rules: [
        {
          test: /\.js$/,
          exclude: /node_modules/,
          use: {
            loader: "babel-loader"
          }
        },
        {
          test: /\.scss$/,
          use: [
              "style-loader", // creates style nodes from JS strings
              "css-loader", // translates CSS into CommonJS
              "sass-loader" // compiles Sass to CSS, using Node Sass by default
          ]
        },
        {
          test: /\.(png|jpe?g|gif)$/,
          use: [
            {
              loader: 'file-loader',
              options: {},
            },
          ],
        },
      ]
    },
    plugins: [
      htmlPlugin,
      faviconPlugin,
    ],
    devServer: {
      historyApiFallback: true,
      proxy: {
        '/api': {
          target: 'http://localhost:8000',
          secure: false,
        },
      },
    },
  };
};