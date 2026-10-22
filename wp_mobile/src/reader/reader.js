/*
 * The script for the Reader's post page, shared by the iOS and Android apps. Put it in the page's
 * `<head>`, so it can hide images before they start to draw.
 *
 * It sends the app these messages, as strings, through a handler named `readerPost`:
 *
 * - `articleTextHighlighted`: the reader selected some text.
 * - `articleTextCopied`: the reader copied some text.
 * - `anchor:<y>:<id>`: the reader tapped a link to an anchor in the post, like a footnote. `y` is
 *   the anchor's distance from the top of the page, in CSS pixels, and `id` is its name, which
 *   `ReaderPostContent.anchors` describes. The app's web view is as tall as the post, so the app
 *   does the scrolling – or shows a footnote's text where the reader is.
 * - `height:<px>`: the page's height, in CSS pixels, whenever it changes: as images and embeds load,
 *   or when the text size changes. The app's web view is as tall as the post, so the app sizes it
 *   to this.
 * - `image:<x>:<y>:<width>:<height>:<url>`: the reader tapped an image that opens in the app's image
 *   viewer: one that isn't a link, or that links to an image file. The numbers are where the image
 *   is on the page, in CSS pixels, so the viewer can open from it. `url` is the image's `src`, as
 *   in `ReaderPostContent.images`.
 * - `blocked:<directive>:<what>`: the page's Content Security Policy blocked something, for the app
 *   to log. `what` is a URL, `inline` or `eval`. An embed that stops working shows up here first.
 *   Each is sent once.
 *
 * On iOS the handler is a `WKScriptMessageHandler`. On Android it's a web message listener.
 */
(() => {
  function post(message) {
    const handler = window.webkit?.messageHandlers?.readerPost ?? window.readerPost;
    handler?.postMessage(message);
  }

  const blocked = new Set();
  document.addEventListener("securitypolicyviolation", (event) => {
    const message = `blocked:${event.effectiveDirective}:${event.blockedURI}`;
    if (!blocked.has(message)) {
      blocked.add(message);
      post(message);
    }
  });

  let height = 0;
  new ResizeObserver(() => {
    const now = Math.ceil(document.documentElement.getBoundingClientRect().height);
    if (now !== height) {
      height = now;
      post(`height:${now}`);
    }
  }).observe(document.documentElement);

  // Show each image once it has loaded, rather than as it draws in. The stylesheet only hides
  // images while this class is on the page, so they still show if the script doesn't run.
  document.documentElement.classList.add("reader-fades-images");

  function show(image) {
    image.classList.add("reader-loaded");
  }

  // Image and iframe `load` events don't bubble, but they do pass through the document on the way
  // down.
  document.addEventListener(
    "load",
    (event) => {
      if (event.target instanceof HTMLImageElement) {
        show(event.target);
      } else if (event.target instanceof HTMLIFrameElement && event.target.dataset.secret) {
        askForHeight(event.target);
      }
    },
    true,
  );

  // Embeds of other WordPress posts. Each is an iframe of the post's embed page, and a blockquote
  // linking to the post in case the iframe can't load. The embed page tells its parent how tall it
  // is, with a secret the iframe and blockquote share. WordPress's own embed script uses that to
  // size the iframe and hide the blockquote; the page doesn't load it, so this does the same.
  function askForHeight(iframe) {
    iframe.contentWindow?.postMessage({ message: "ready", secret: iframe.dataset.secret }, "*");
  }

  window.addEventListener("message", (event) => {
    const { message, value, secret } = event.data ?? {};
    if (typeof secret !== "string" || !/^[a-zA-Z0-9]+$/.test(secret)) {
      return;
    }
    const iframe = Array.from(document.querySelectorAll(`iframe[data-secret="${secret}"]`)).find(
      (frame) => frame.contentWindow === event.source,
    );
    if (!iframe) {
      return;
    }
    for (const blockquote of document.querySelectorAll(`blockquote[data-secret="${secret}"]`)) {
      blockquote.style.display = "none";
    }

    if (message === "height") {
      // WordPress keeps embeds between 200 and 1,000 pixels tall.
      const height = Math.min(Math.max(parseInt(value, 10) || 0, 200), 1000);
      iframe.style.aspectRatio = "auto";
      iframe.style.height = `${height}px`;
    } else if (message === "link") {
      // A link the reader tapped in the embed, to somewhere on the embedded post's site. It opens
      // the way the post's own links do.
      try {
        const target = new URL(value);
        const isOnTheSameSite = new URL(iframe.src).host === target.host;
        if (/^https?:$/.test(target.protocol) && isOnTheSameSite && document.activeElement === iframe) {
          window.location.href = target.href;
        }
      } catch {
        // Not a URL.
      }
    }
  });

  // GitHub gists. The cleanup makes each one a code block, with the address of the gist's data:
  // its HTML, and GitHub's stylesheet for it. GitHub only serves that to other sites as JSONP, a
  // script that calls the function its `callback` names. Once the stylesheet has loaded, the gist
  // takes the code block's place, so it never shows unstyled. Until then, or if GitHub doesn't
  // answer, the code block stays.
  const gistStylesheets = new Map();

  function loadGistStylesheet(href) {
    if (!gistStylesheets.has(href)) {
      const loaded = new Promise((resolve, reject) => {
        const link = document.createElement("link");
        link.rel = "stylesheet";
        link.href = href;
        link.addEventListener("load", resolve, { once: true });
        link.addEventListener("error", reject, { once: true });
        document.head.append(link);
      });
      gistStylesheets.set(href, loaded);
    }
    return gistStylesheets.get(href);
  }

  async function showGist(codeBlock, gist) {
    let stylesheet;
    try {
      stylesheet = new URL(gist?.stylesheet);
    } catch {
      return;
    }
    if (stylesheet.origin !== "https://github.githubassets.com" || typeof gist.div !== "string") {
      return;
    }
    try {
      await loadGistStylesheet(stylesheet.href);
    } catch {
      // Without its stylesheet, the gist looks worse than the code block.
      return;
    }
    const template = document.createElement("template");
    template.innerHTML = gist.div;
    codeBlock.replaceWith(template.content);
  }

  document.addEventListener("DOMContentLoaded", () => {
    const codeBlocks = document.querySelectorAll("figure.reader-gist[data-reader-gist]");
    codeBlocks.forEach((codeBlock, index) => {
      let url;
      try {
        url = new URL(codeBlock.dataset.readerGist);
      } catch {
        return;
      }
      if (url.origin !== "https://gist.github.com") {
        return;
      }
      const callback = `readerGist${index}`;
      const script = document.createElement("script");
      const done = () => {
        delete window[callback];
        script.remove();
      };
      window[callback] = (gist) => {
        done();
        showGist(codeBlock, gist);
      };
      script.addEventListener("error", done);
      url.searchParams.set("callback", callback);
      script.src = url.href;
      document.head.append(script);
    });
  });

  document.addEventListener("DOMContentLoaded", () => {
    for (const image of Array.from(document.images)) {
      // A lazy `sizes="auto"` image can report `complete` before it has loaded.
      if (image.complete && image.naturalWidth > 0) {
        show(image);
      }

      // An image that isn't a link becomes a link to itself, so a tap opens it in the app's image
      // viewer (below), and it can be opened like a link. Images that are already links keep their
      // link, and emoji and Story images aren't for opening.
      if (!image.closest("a") && !image.matches(".emoji, .wp-smiley, .wp-story-image")) {
        // A `<picture>` has to keep its `<img>`, so the link goes around the whole thing.
        const media = image.parentElement?.matches("picture") ? image.parentElement : image;
        const link = document.createElement("a");
        link.href = image.src;
        link.dataset.readerImage = "";
        media.replaceWith(link);
        link.append(media);
      }
    }
  });

  // Taps on images that open in the app's image viewer: the ones this script made links, and ones the
  // post links to an image file. Their links go nowhere; the app opens the viewer.
  const imageFile = /\.(jpe?g|jpe|png|gif|webp|avif|heic|heif|bmp|tiff?)$/i;
  function linksToAnImage(link) {
    try {
      return imageFile.test(new URL(link.href).pathname);
    } catch {
      return false;
    }
  }

  document.addEventListener(
    "click",
    (event) => {
      const image = event.target instanceof HTMLImageElement ? event.target : null;
      const src = image?.getAttribute("src");
      if (!src || image.matches(".emoji, .wp-smiley, .wp-story-image")) {
        return;
      }
      const link = image.closest("a");
      if (link && !("readerImage" in link.dataset) && !linksToAnImage(link)) {
        return;
      }
      event.preventDefault();
      const box = image.getBoundingClientRect();
      const place = [box.left + window.scrollX, box.top + window.scrollY, box.width, box.height];
      post(`image:${place.map(Math.round).join(":")}:${src}`);
    },
    true,
  );

  // Text selection, for analytics. Selecting text fires a lot of these, so wait for it to settle.
  let selectionTimer;
  document.addEventListener("selectionchange", () => {
    if (document.getSelection().toString().length > 0) {
      clearTimeout(selectionTimer);
      selectionTimer = setTimeout(() => post("articleTextHighlighted"), 1000);
    }
  });

  document.addEventListener("copy", () => post("articleTextCopied"));

  // Links to somewhere else in the post, like footnotes.
  document.addEventListener(
    "click",
    (event) => {
      const link = event.target instanceof Element && event.target.closest('a[href^="#"]');
      if (!link) {
        return;
      }
      // It's a link within the post, so it never goes anywhere else, even when what it links to
      // is missing. The Reader API leaves out the footnotes block, but not the links to it.
      event.preventDefault();

      let id = link.getAttribute("href").slice(1);
      try {
        id = decodeURIComponent(id);
      } catch {
        // Not percent-encoded after all.
      }
      const target = document.getElementById(id) ?? document.getElementsByName(id)[0];
      if (!target) {
        return;
      }

      const y = Math.round(target.getBoundingClientRect().top + window.scrollY);
      post(`anchor:${y}:${id}`);
    },
    true,
  );
})();
