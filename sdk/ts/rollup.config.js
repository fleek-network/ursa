import typescript from '@rollup/plugin-typescript'
import { nodeResolve } from '@rollup/plugin-node-resolve'
import commonjs from '@rollup/plugin-commonjs'
import babel from '@rollup/plugin-babel'
import json from '@rollup/plugin-json'
import globals from 'rollup-plugin-node-globals'

import pkg from './package.json' assert { type: 'json' }

export default {
    input: `src/index.ts`,
    output: [
        { file: pkg.main, name: 'ursa-sdk', format: 'umd', sourcemap: true },
        { file: pkg.module, format: 'es', sourcemap: true }
    ],
    watch: {
        include: 'src/**'
    },
    plugins: [
        typescript({ noEmitOnError: false, outDir: 'dist/' }),
        babel({
            exclude: ['/node_modules'],
            presets: ['@babel/preset-typescript'],
            extensions: ['.ts', '.tsx']
        }),
        nodeResolve({
            jsnext: true,
            extensions: ['.ts', '.js', '.json']
        }),
        commonjs(),
        globals(),
        json()
    ]
}
