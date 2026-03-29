import { useState, useCallback } from 'react';
import en from './en.json';
import es from './es.json';
import pt from './pt.json';

const translations: Record<string, Record<string, any>> = { en, es, pt };
const STORAGE_KEY = 'agentos_language';

type Language = 'en' | 'es' | 'pt';

function getNestedValue(obj: any, path: string): string {
    return path.split('.').reduce((acc, key) => acc?.[key], obj) || path;
}

function detectLanguage(): Language {
    // Check localStorage first
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored && translations[stored]) return stored as Language;

    // Check browser locale
    const browserLang = navigator.language.split('-')[0];
    if (translations[browserLang]) return browserLang as Language;

    return 'en';
}

export function useTranslation() {
    const [language, setLanguageState] = useState<Language>(detectLanguage);

    const setLanguage = useCallback((lang: Language) => {
        setLanguageState(lang);
        localStorage.setItem(STORAGE_KEY, lang);
    }, []);

    const t = useCallback((key: string): string => {
        return getNestedValue(translations[language], key);
    }, [language]);

    return { t, language, setLanguage, availableLanguages: ['en', 'es', 'pt'] as const };
}

export type { Language };
export { detectLanguage };
