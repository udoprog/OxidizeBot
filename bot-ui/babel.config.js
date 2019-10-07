module.exports = (api) => {
    api.cache(true);

    const presets = [
        [
            "@babel/env",
            {
                targets: "last 2 versions",
                useBuiltIns: "usage",
                corejs: 3
            }
        ],
        "@babel/react"
    ];

    return { presets };
};