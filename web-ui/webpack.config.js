const path = require('path');
const glob = require('glob')

const HtmlWebPackPlugin = require("html-webpack-plugin");
const DynamicCdnWebpackPlugin = require('dynamic-cdn-webpack-plugin');
const FaviconsWebpackPlugin = require('favicons-webpack-plugin')

const htmlPlugin = new HtmlWebPackPlugin({
  template: "./src/index.html",
  filename: "./index.html"
});

const cdn = new DynamicCdnWebpackPlugin();
const faviconPlugin = new FaviconsWebpackPlugin("../bot/res/icon.png");
const MiniCssExtractPlugin = require('mini-css-extract-plugin');

module.exports = function(_, argv) {
  return {
    resolveLoader: {
      modules: [
        'node_modules',
        path.resolve(__dirname, 'loaders')
      ]
    },
    output: {
      path: path.join(__dirname, 'dist'),
      filename: '[chunkhash].js',
      chunkFilename: '[chunkhash].js'
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
              {
                loader: MiniCssExtractPlugin.loader,
                options: {
                  hmr: process.env.NODE_ENV === 'development',
                },
              },
              'css-loader',
              {
                loader: 'sass-loader',
                options: {
                  implementation: require('sass'),
                  sassOptions: {
                    fiber: false,
                  },
                },
              },
          ]
        },
        {
          test: /\.(png|jpe?g|gif|svg)$/,
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
      cdn,
      new MiniCssExtractPlugin({
        filename: '[name].[hash].css',
        chunkFilename: '[id].[hash].css',
      })
    ],
    devServer: {
      historyApiFallback: true,
      proxy: {
        '/api': {
          target: 'https://setbac.tv',
          secure: false,
        },
      },
    },
  };
};