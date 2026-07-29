type OriginalLocale = "ko" | "ja" | "en" | "zh-TW" | "zh-CN" | "ru" | "fr" | "es" | "pt" | "it" | "de" | "th" | "vi" | "id";

export const ORIGINAL_UI_LABEL_EVIDENCE = {
  source: "sharedassets0.assets",
  pathId: 9082,
  worksheet: "localUI",
  contract: "reverse-engineering/evidence/building-ui-contract-v1.json",
} as const;

const labels: Record<string, Record<OriginalLocale, string>> = {
  btn_0: { ko: "닫기", ja: "閉じる", en: "Close", "zh-TW": "關閉", "zh-CN": "关闭", ru: "Закрыть", fr: "Fermer", es: "Cerrar", pt: "Fechar", it: "Chiudere", de: "Schließen", th: "ปิด", vi: "Đóng", id: "Tutup" },
  buildpop_2: { ko: "요금표", ja: "料金表", en: "Cost", "zh-TW": "收費表", "zh-CN": "收费表", ru: "Цена", fr: "Coût", es: "Coste", pt: "Custo", it: "Costo", de: "Kosten", th: "ตารางราคา", vi: "Biểu giá", id: "Biaya" },
  buildpop_7: { ko: "업그레이드", ja: "アップグレード", en: "Upgrade", "zh-TW": "升級", "zh-CN": "升级", ru: "Улучшить", fr: "Améliorer", es: "Subir de Nivel", pt: "Aprimorar", it: "Aggiornamento", de: "Aufwertung", th: "อัปเกรด", vi: "Nâng Cấp", id: "Tingkatkan" },
  buildpop_9: { ko: "<color=#D97C7C>Lv.{0}이상 마을회관 필요</color>", ja: "<color=#D97C7C>Lv.{0}以上の町内会館必要 </color>", en: "<color=#D97C7C>Town Hall Lv.{0} or higher required.</color>", "zh-TW": "<color=#D97C7C>需要Lv.{0}以上主城會館</color>", "zh-CN": "<color=#D97C7C>需要Lv.{0}以上主城会馆</color>", ru: "<color=#D97C7C>Требуется Ратуша Ур.{0} или выше.</color>", fr: "<color=#D97C7C>Mairie Nv.{0} ou supérieur requise.</color>", es: "Se requiere el<color=#D97C7C>Niv.de Alcaldía {0} o más.</color>", pt: "<color=#D97C7C>Necessário Prefeitura Nv.{0} ou acima.</color>", it: "<color=#D97C7C>Richiesto LIV Municipio {0} o superiore.</color>", de: "<color=#D97C7C>Stadthalle Lv.{0} oder höher benötigt.</color>", th: "<color=#D97C7C>ต้องการหอประชุมหมู่บ้านเลเวล Lv.{0} ขึ้นไป</color>", vi: "<color=#D97C7C>Cần Tòa Thị Chính từ Lv.{0} trở lên</color>", id: "<color=#D97C7C>Dibutuhkan Balai Kota Lv.{0} atau lebih tinggi.</color>" },
  buildpop_25: { ko: "구매예약", ja: "購入予約", en: "Request", "zh-TW": "預購", "zh-CN": "预购", ru: "Запрос", fr: "Requête", es: "Solicitar", pt: "Pedir", it: "Richiesta", de: "Anfrage", th: "จอง", vi: "Đặt mua", id: "Meminta" },
  buildpop_26: { ko: "구매예약 {0}건", ja: "購入予約 {0}件", en: "Request to Purchase: {0}", "zh-TW": "預購{0}件", "zh-CN": "预购{0}件", ru: "Запрос на покупку: {0}", fr: "Requête à Acheter : {0}", es: "Solicitar para Comprar: {0}", pt: "Pedido de compras: {0}", it: "Richiesta di acquisto: {0}", de: "Kaufsanfrage: {0}", th: "จอง  {0} ชิ้น", vi: "Đặt mua: {0}", id: "Meminta untuk Beli: {0}" },
  buildpop_32: { ko: "필요자원 ", ja: "必要資源", en: "Required Resources", "zh-TW": "必要資源", "zh-CN": "必要资源", ru: "Необходимые Ресурсы", fr: "Ressources Requises", es: "Recursos Necesarios", pt: "Recursos Necessários", it: "Risorse Necessarie", de: "Benötigte Ressourcen", th: "ทรัพยากรที่ต้องการ", vi: "Nguyên Liệu Yêu Cầu", id: "Sumber Daya yang Dibutuhkan" },
};

export function originalUiLabel(key: keyof typeof labels | string, locale = "vi", args: Array<string | number> = []): string {
  const translations = labels[key];
  if (!translations) return key;
  const normalized = normalizeLocale(locale);
  const template = translations[normalized] ?? translations[normalized.split("-")[0] as OriginalLocale] ?? translations.en;
  return stripUnityRichText(args.reduce<string>((value, argument, index) => value.replaceAll(`{${index}}`, String(argument)), template));
}

function normalizeLocale(locale: string): OriginalLocale {
  const normalized = locale.replace("_", "-");
  if (normalized.toLowerCase() === "zh-tw") return "zh-TW";
  if (normalized.toLowerCase().startsWith("zh")) return "zh-CN";
  const language = normalized.split("-")[0] as OriginalLocale;
  return language in labels.btn_0 ? language : "en";
}

function stripUnityRichText(value: string): string {
  return value.replace(/<\/?color(?:=[^>]+)?>/g, "");
}
