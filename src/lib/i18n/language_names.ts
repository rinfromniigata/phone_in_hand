/**
 * 言語の選択肢の表示名。どの言語で表示していても、その言語自身の名前（自称）で出す。
 * 利用者が、読めない言語から抜け出せるようにするため、辞書には置かない。
 */
export const LANGUAGE_NAMES: Record<string, string> = {
	ja: '日本語',
	en: 'English',
	'zh-Hans': '简体中文'
};

/** 名前が決まっていないロケール（開発時だけの疑似ロケール）は、コードをそのまま出す */
export const languageName = (locale: string): string => LANGUAGE_NAMES[locale] ?? locale;
