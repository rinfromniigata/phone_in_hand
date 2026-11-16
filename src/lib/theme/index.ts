// 読み込む順序に意味がある：Carbon（@layer carbon）→ 書体 → トークン → 土台
import 'carbon-components-svelte/css/all.css';
import './fonts.css';
import './tokens.css';
import './base.css';

export { applyTheme, resolveTheme, type ResolvedTheme, type ThemeSetting } from './theme.ts';
