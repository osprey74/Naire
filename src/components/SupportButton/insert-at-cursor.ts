export interface InsertResult {
  value: string;
  cursor: number;
}

/// 入力欄のカーソル位置に文字列を挿入する。選択範囲があれば置換する。
/// フォーカス未取得などで `selectionStart` が `null` の場合は末尾に追記する。
export function insertAtCursor(
  input: HTMLInputElement | HTMLTextAreaElement,
  text: string,
): InsertResult {
  const start = input.selectionStart ?? input.value.length;
  const end = input.selectionEnd ?? input.value.length;
  const value = input.value.slice(0, start) + text + input.value.slice(end);
  const cursor = start + text.length;
  return { value, cursor };
}
