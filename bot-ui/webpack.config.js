const path = require("path");
const HtmlWebPackPlugin = require("html-webpack-plugin");
const TerserPlugin = require("terser-webpack-plugin");
const devMode = process.env.NODE_ENV !== "production";

module.exports = function (_, argv) {
  return {
    output: {
      filename: '[name].[contenthash].js',
      path: path.join(__dirname, 'dist')
    },
    optimization: {
      splitChunks: { chunks: "all" },
      minimize: !devMode,
      minimizer: [
        new TerserPlugin({
          extractComments: false,
        }),
      ],
    },
    module: {
      rules: [
        {
          test: /\.js$/,
          include: /node_modules/,
          type: 'javascript/auto'
        },
        {
          test: /\.js$/,
          exclude: /node_modules/,
          use: {
            loader: "babel-loader",
            options: {
              presets: [
                ['@babel/preset-env', {
                  useBuiltIns: 'usage',
                  corejs: "3",
                  targets: {
                    browsers: ['last 2 versions', 'ie >= 9'],
                  },
                }],
                '@babel/react'
              ]
            }
          }
        },
        {
          test: /\.scss$/,
          use: [
            "style-loader",
            "css-loader",
            {
              loader: 'sass-loader',
              options: {
                implementation: require('sass'),
                sassOptions: {
                  fiber: false,
                },
              },
            }
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
      new HtmlWebPackPlugin({
        favicon: "./static/favicon.ico",
        template: "./src/index.html",
        filename: "./index.html"
      }),
    ],
    devServer: {
      historyApiFallback: true,
      proxy: {
        '/ws': {
          target: 'ws://localhost:12345',
          secure: false,
          ws: true,
        },
        '/api': {
          target: 'http://localhost:12345',
          secure: false,
        },
      },
    }
  };
};
