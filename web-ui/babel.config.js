module.exports = (api) => {
    api.cache(true);

    const presets = [
        [
            "@babel/preset-env",
            {
                targets: "last 2 versions",
                useBuiltIns: "usage",
                corejs: 3
            }
        ],
        "@babel/preset-react"
    ];

    return { presets };
};