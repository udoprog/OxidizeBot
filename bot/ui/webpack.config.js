const HtmlWebPackPlugin = require("html-webpack-plugin");

const htmlPlugin = new HtmlWebPackPlugin({
  template: "./src/index.html",
  filename: "./index.html"
});

module.exports = {
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
      }
    ]
  },
  plugins: [htmlPlugin],
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