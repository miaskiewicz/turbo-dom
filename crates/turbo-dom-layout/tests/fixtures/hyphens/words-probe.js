// turbo-dom#289: `hyphens: auto` break points per language. For each word (Inter 16px,
// the element `lang`, default `hyphenate-limit-chars`) the box width is swept a CSS pixel at a
// time; every width whose first line ends inside the word records that break (the character
// index the next line starts at). Chrome breaks at the last point that fits, and each letter is
// wider than a pixel, so the sweep finds every point the dictionary offers under the limits.
(() => {
  const words = {
    "en": ["international", "extraordinary", "typography", "characteristics", "responsibility", "performance", "information", "development", "government", "communication"],
    "de": ["Silbentrennung", "Rechtschreibung", "wunderbar", "Geschwindigkeit", "Entwicklung", "Bundesrepublik", "Wissenschaft", "Kindergarten", "Zusammenarbeit", "Verantwortung", "Unterschied", "Bibliothek", "Straßenbahn", "Universität", "Möglichkeiten", "Geschichte", "Gesellschaft", "Wirtschaft", "Regierung", "Fahrradweg", "schwierig", "Freundschaft", "Überraschung", "Eichhörnchen", "Schmetterling", "Krankenhaus", "Nachrichten", "Erfahrung", "Tageszeitung", "Programmierung"],
    "fr": ["international", "extraordinaire", "anticonstitutionnellement", "développement", "gouvernement", "responsabilité", "bibliothèque", "connaissance", "particulièrement", "information", "république", "architecture", "conversation", "professionnel", "technologie", "environnement", "malheureusement", "communication", "imagination", "philosophie", "nécessaire", "littérature", "entreprise", "historique", "complètement", "organisation", "différence", "quotidien", "électricité", "quotidiennement"],
    "es": ["internacional", "extraordinario", "murciélago", "desarrollo", "gobierno", "responsabilidad", "biblioteca", "conocimiento", "particularmente", "información", "república", "arquitectura", "conversación", "profesional", "tecnología", "comunicación", "imaginación", "filosofía", "necesario", "literatura", "empresa", "histórico", "completamente", "organización", "diferencia", "electricidad", "ferrocarril", "pingüino", "acción", "guerrero"],
    "nl": ["ziekenhuis", "onafhankelijkheid", "ontwikkeling", "regering", "verantwoordelijkheid", "bibliotheek", "kennis", "informatie", "republiek", "architectuur", "gesprek", "professioneel", "technologie", "communicatie", "verbeelding", "filosofie", "noodzakelijk", "literatuur", "onderneming", "geschiedenis", "volledig", "organisatie", "verschil", "elektriciteit", "fietspad", "kinderwagen", "boterham", "schilderij", "wetenschap", "vriendschap"],
    "pl": ["międzynarodowy", "konstantynopolitańczykowianeczka", "rozwój", "rząd", "odpowiedzialność", "biblioteka", "wiedza", "szczególnie", "informacja", "rzeczpospolita", "architektura", "rozmowa", "profesjonalny", "technologia", "komunikacja", "wyobraźnia", "filozofia", "konieczny", "literatura", "przedsiębiorstwo", "historyczny", "całkowicie", "organizacja", "różnica", "elektryczność", "dziewczyna", "przyjaciel", "chrząszcz", "samochód", "uniwersytet"],
    "en-GB": ["international", "extraordinary", "typography", "characteristics", "responsibility", "performance", "information", "development", "government", "communication", "organisation", "philosophy", "technology", "beautiful", "computer", "example", "hyphenation", "literature", "particularly", "imagination"],
    "it": ["internazionale", "straordinario", "sviluppo", "governo", "responsabilità", "biblioteca", "conoscenza", "informazione", "architettura", "conversazione"],
    "pt": ["internacional", "extraordinário", "desenvolvimento", "governo", "responsabilidade", "biblioteca", "conhecimento", "informação", "arquitetura", "conversação"],
    "sv": ["internationell", "utveckling", "regering", "ansvar", "bibliotek", "kunskap", "information", "arkitektur", "samtal", "sjukhus"],
    "ru": ["международный", "развитие", "правительство", "ответственность", "библиотека", "знание", "информация", "архитектура", "разговор", "университет"],
    "da": ["international", "udvikling", "regering", "ansvar", "bibliotek", "viden", "information", "arkitektur", "samtale", "sygehus"],
    "nb": ["internasjonal", "utvikling", "regjering", "ansvar", "bibliotek", "kunnskap", "informasjon", "arkitektur", "samtale", "sykehus"],
    "fi": ["kansainvälinen", "kehitys", "hallitus", "vastuu", "kirjasto", "tieto", "informaatio", "arkkitehtuuri", "keskustelu", "sairaala"],
    "cs": ["mezinárodní", "rozvoj", "vláda", "odpovědnost", "knihovna", "znalost", "informace", "architektura", "rozhovor", "nemocnice"],
    "hu": ["nemzetközi", "fejlesztés", "kormány", "felelősség", "könyvtár", "tudás", "információ", "építészet", "beszélgetés", "kórház"],
    "uk": ["міжнародний", "розвиток", "уряд", "відповідальність", "бібліотека", "знання", "інформація", "архітектура", "розмова", "університет"],
    "ca": ["internacional", "desenvolupament", "govern", "responsabilitat", "biblioteca", "coneixement", "informació", "arquitectura", "conversa", "hospital"],
    "hr": ["međunarodni", "razvoj", "vlada", "odgovornost", "knjižnica", "znanje", "informacija", "arhitektura", "razgovor", "bolnica"],
    "sk": ["medzinárodný", "rozvoj", "vláda", "zodpovednosť", "knižnica", "znalosť", "informácia", "architektúra", "rozhovor", "nemocnica"],
    "ro": ["internațional", "dezvoltare", "guvern", "responsabilitate", "bibliotecă", "cunoaștere", "informație", "arhitectură", "conversație", "spital"],
    "el": ["διεθνής", "ανάπτυξη", "κυβέρνηση", "ευθύνη", "βιβλιοθήκη", "γνώση", "πληροφορία", "αρχιτεκτονική", "συζήτηση", "νοσοκομείο"],
  };
  const out = {};
  for (const [lang, list] of Object.entries(words)) {
    out[lang] = {};
    for (const word of list) {
      const div = document.createElement("div");
      div.setAttribute("style", "font-family:Inter;font-size:16px;line-height:20px;hyphens:auto");
      div.lang = lang;
      div.textContent = word;
      document.body.append(div);
      const text = div.firstChild;
      const range = document.createRange();
      const top = (i) => {
        range.setStart(text, i);
        range.setEnd(text, i + 1);
        const rects = range.getClientRects();
        return rects[rects.length - 1].top;
      };
      range.selectNodeContents(div);
      const full = Math.ceil(range.getBoundingClientRect().width);
      const breaks = new Set();
      for (let width = 1; width <= full + 16; width++) {
        div.style.width = `${width}px`;
        const first = top(0);
        for (let i = 1; i < word.length; i++)
          if (top(i) !== first) {
            breaks.add(i);
            break;
          }
      }
      out[lang][word] = [...breaks].sort((a, b) => a - b);
      div.remove();
    }
  }
  return JSON.stringify({ words: out });
})();
