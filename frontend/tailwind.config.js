/** @type {import('tailwindcss').Config} */
export default {
    content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
    theme: {
        extend: {
            colors: {
                bg: {
                    primary: '#0A0E14',
                    surface: '#0D1117',
                    deep: '#080B10',
                    elevated: '#1A1E26',
                },
                cyan: {
                    DEFAULT: '#00E5E5',
                    dark: '#00B8D4',
                    muted: '#4DB8B8',
                },
                text: {
                    primary: '#E6EDF3',
                    secondary: '#C5D0DC',
                    muted: '#3D4F5F',
                    dim: '#2A3441',
                },
                success: '#2ECC71',
                error: '#E74C3C',
                warning: '#F39C12',
                info: '#378ADD',
                purple: '#5865F2',
            },
            fontFamily: {
                sans: ['Inter', 'system-ui', 'sans-serif'],
                mono: ['JetBrains Mono', 'monospace'],
            },
        },
    },
    plugins: [],
};
