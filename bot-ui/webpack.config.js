const HtmlWebPackPlugin = require("html-webpack-plugin");
const FaviconsWebpackPlugin = require('favicons-webpack-plugin')

const htmlPlugin = new HtmlWebPackPlugin({
  template: "./src/index.html",
  filename: "./index.html"
});

const faviconPlugin = new FaviconsWebpackPlugin("../bot/res/icon.png");

module.exports = function(_, argv) {
  return {
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
        '/ws/*': {
          target: 'ws://localhost:12345',
          secure: false,
          ws: true,
        },
        '/api': {
          target: 'http://localhost:12345',
          secure: false,
        },
      },
    },
  };
};