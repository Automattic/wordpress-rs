#if canImport(WebKit) && (os(macOS) || os(iOS))
import Foundation
import Testing
import WebKit
import WordPressAPIInternal

/// Runs `readerScript()` in a web view, against a page with one of everything it handles. The page
/// checks the script's behaviour itself, and lists the result of each check in `#results`.
@MainActor
struct ReaderScriptTests {
    @Test
    func behavesAsTheAppsExpect() async throws {
        let webView = WKWebView(frame: CGRect(x: 0, y: 0, width: 390, height: 800))
        webView.loadHTMLString(Self.page, baseURL: nil)

        var results = "pending"
        for _ in 0..<150 where results == "pending" {
            try await Task.sleep(for: .milliseconds(200))
            let value = try? await webView.evaluateJavaScript("document.getElementById('results')?.textContent")
            results = value as? String ?? "pending"
        }

        #expect(results != "pending", "the checks didn't finish")
        let failures = results.split(separator: "\n").filter { !$0.hasPrefix("PASS") }
        #expect(failures.isEmpty, "\(failures.joined(separator: "\n"))")
    }

    /// The page from `pageHtml` has a Content Security Policy. The shared script runs under it, and
    /// tells the app about anything it blocks.
    @Test
    func runsUnderThePagePolicy() async throws {
        let recorder = MessageRecorder()
        let configuration = WKWebViewConfiguration()
        configuration.userContentController.add(recorder, name: "readerPost")
        let webView = WKWebView(frame: CGRect(x: 0, y: 0, width: 390, height: 800), configuration: configuration)
        let options = ReaderPageOptions(theme: .sepia, appColors: readerDefaultAppColors(brand: .wordpress))
        let page = ReaderPostContent(html: "<p>Hi</p>").pageHtml(options: options)
        webView.loadHTMLString(page, baseURL: URL(string: "https://wordpress.com/reader"))

        // The shared script marks the page as soon as it runs.
        var ran = false
        for _ in 0..<50 where !ran {
            try await Task.sleep(for: .milliseconds(100))
            let value = try? await webView.evaluateJavaScript(
                "document.documentElement.classList.contains('reader-fades-images')"
            )
            ran = value as? Bool ?? false
        }
        #expect(ran, "the policy blocked the shared script")

        // The app's own JavaScript isn't subject to the page's policy, but scripts it adds to the page are.
        _ = try? await webView.evaluateJavaScript(
            """
            const inline = document.createElement('script');
            inline.textContent = 'document.title = "ran"';
            document.head.append(inline);
            const external = document.createElement('script');
            external.src = 'https://example.com/a.js';
            document.head.append(external);
            0
            """
        )
        let blocked = { recorder.messages.filter { $0.hasPrefix("blocked:") } }
        for _ in 0..<50 where blocked().count < 2 {
            try await Task.sleep(for: .milliseconds(100))
        }
        let title = try? await webView.evaluateJavaScript("document.title")
        #expect(title as? String != "ran", "the policy let an inline script run")
        #expect(
            Set(blocked()) == ["blocked:script-src-elem:inline", "blocked:script-src-elem:https://example.com/a.js"]
        )
        #expect(recorder.messages.contains { $0.hasPrefix("height:") }, "the page didn't report its height")
    }

    /// The shared script loads each gist's data from GitHub, and puts the gist in its code block's
    /// place once GitHub's stylesheet has loaded. Without the network: the page's policy blocks
    /// GitHub, and the page answers for it before the blocked load fails.
    @Test
    func loadsGistsInTheirCodeBlocks() async throws {
        let webView = WKWebView(frame: CGRect(x: 0, y: 0, width: 390, height: 800))
        webView.loadHTMLString(Self.gistPage, baseURL: URL(string: "https://wordpress.com/reader"))

        var results = "pending"
        for _ in 0..<50 where results == "pending" {
            try await Task.sleep(for: .milliseconds(200))
            let value = try? await webView.evaluateJavaScript("document.getElementById('results')?.textContent")
            results = value as? String ?? "pending"
        }

        #expect(results != "pending", "the checks didn't finish")
        let failures = results.split(separator: "\n").filter { !$0.hasPrefix("PASS") }
        #expect(failures.isEmpty, "\(failures.joined(separator: "\n"))")
    }

    static let gistPage = """
        <!DOCTYPE html><html><head><meta charset="UTF-8">
        <meta http-equiv="Content-Security-Policy" content="script-src 'unsafe-inline'; style-src 'unsafe-inline'">
        <script>\(gistGitHub)</script>
        <script>\(readerScript())</script>
        <style>\(readerStylesheet())</style>
        </head><body>
        <p id="before">Before</p>
        \(gistCodeBlock("gist", data: "\(gistData)?file=hello_world.rb"))
        \(gistCodeBlock("second", data: gistData))
        \(gistCodeBlock("stalled", data: gistData))
        \(gistCodeBlock("other-stylesheet", data: gistData))
        \(gistCodeBlock("elsewhere", data: "https://example.com/a.json"))
        <pre id="results">pending</pre>
        <script>\(gistChecks)</script>
        </body></html>
        """

    static let gistData = "https://gist.github.com/octocat/6cad326836d38bd3a7ae.json"

    /// A code block the cleanup makes of a gist.
    static func gistCodeBlock(_ id: String, data: String) -> String {
        "<figure id=\"\(id)\" class=\"reader-gist\" data-reader-gist=\"\(data)\">"
            + "<pre><code>class HelloWorld</code></pre></figure>"
    }

    /// Stands in for GitHub: answers for the gists the checks expect to load, and loads GitHub's
    /// stylesheet, as soon as the shared script asks. The page's policy blocks both, but only after
    /// this has answered.
    static let gistGitHub = #"""
        const messages = [];
        window.readerPost = { postMessage: (message) => messages.push(message) };
        const requested = [];
        const stylesheet = 'https://github.githubassets.com/assets/gist-embed-test.css';
        const answers = {
          readerGist0: { div: '<div id="gist-from-github" class="gist">The gist</div>', stylesheet },
          readerGist1: { div: '<div id="second-from-github" class="gist">The gist</div>', stylesheet },
          // readerGist2 never answers.
          readerGist3: {
            div: '<div id="unstyled-from-github" class="gist">The gist</div>',
            stylesheet: 'https://example.com/a.css',
          },
        };
        new MutationObserver((records) => {
          for (const node of records.flatMap((record) => Array.from(record.addedNodes))) {
            if (node instanceof HTMLScriptElement && node.src) {
              requested.push(node.src);
              const callback = new URL(node.src).searchParams.get('callback');
              if (answers[callback]) {
                window[callback](answers[callback]);
              }
            } else if (node instanceof HTMLLinkElement && node.href === stylesheet) {
              node.dispatchEvent(new Event('load'));
            }
          }
        }).observe(document.documentElement, { childList: true, subtree: true });
        """#

    static let gistChecks = #"""
        const results = [];
        const check = (name, ok, detail = '') => {
          results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${ok ? '' : ' – ' + detail}`);
        };
        const byId = (id) => document.getElementById(id);

        window.addEventListener('load', async () => {
          await new Promise((resolve) => setTimeout(resolve, 300));

          const first = 'https://gist.github.com/octocat/6cad326836d38bd3a7ae.json'
            + '?file=hello_world.rb&callback=readerGist0';
          check(
            'a gist\'s code block asks GitHub for its data, with a callback',
            requested[0] === first,
            JSON.stringify(requested)
          );
          check(
            'a code block whose data is somewhere else asks for nothing',
            requested.length === 4 && !requested.some((src) => src.includes('example.com')),
            JSON.stringify(requested)
          );
          check(
            'the gist takes its code block\'s place',
            !byId('gist') && byId('gist-from-github')?.previousElementSibling === byId('before'),
            document.body.innerHTML.slice(0, 300)
          );
          check('a second gist does too', !byId('second') && byId('second-from-github'));
          check(
            'GitHub\'s stylesheet is only added once',
            document.querySelectorAll(`link[href="${stylesheet}"]`).length === 1
          );
          check('the code block stays when GitHub doesn\'t answer', byId('stalled') !== null);
          check(
            'the code block stays when the stylesheet isn\'t GitHub\'s',
            byId('other-stylesheet') !== null && !byId('unstyled-from-github')
          );
          check(
            'the app hears that the page\'s policy blocked GitHub',
            messages.some((message) => message.startsWith('blocked:script-src-elem:https://gist.github.com/')),
            JSON.stringify(messages)
          );
          check(
            'the callbacks are gone once GitHub has answered, or failed',
            ['readerGist0', 'readerGist1', 'readerGist2', 'readerGist3'].every((name) => !(name in window)),
            Object.keys(window).filter((name) => name.startsWith('readerGist')).join(', ')
          );

          byId('results').textContent = results.join('\n');
        });
        """#

    /// A 2×2 PNG, so images load without the network.
    static let png =
        "data:image/png;base64,"
        + "iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAIAAAD91JpzAAAAFklEQVR4nGP8z8DAwMDAxMDAwMDAAAANHQEDasKb6QAAAABJRU5ErkJggg=="

    static let page = """
        <!DOCTYPE html><html><head><meta charset="UTF-8">
        <script>
          const messages = [];
          const heights = [];
          window.readerPost = {
            postMessage: (message) => (message.startsWith('height:') ? heights : messages).push(message),
          };
        </script>
        <script>\(readerScript())</script>
        <style>\(readerStylesheet())</style>
        </head><body>
        \(markup)
        <pre id="results">pending</pre>
        <script>\(checks)</script>
        </body></html>
        """

    static let markup = """
        <p id="text">Some text to select.</p>
        <p><img id="plain" src="\(png)" width="20" height="20"></p>
        <p><img id="broken" src="file:///nonexistent/missing.png" width="20" height="20"></p>
        <p><a href="https://example.com/post"><img id="linked" src="\(png)"></a></p>
        <p><a href="https://example.com/full.jpg"><img id="file-linked" src="\(png)"></a></p>
        <p>An emoji <img id="emoji" class="emoji" src="\(png)"></p>
        <p><picture><source srcset="\(png)"><img id="pictured" src="\(png)"></picture></p>
        <p>
          <a id="to-footnote" href="#fn-1">1</a>
          <a id="to-named" href="#named">named</a>
          <a id="to-nowhere" href="#nowhere">nowhere</a>
        </p>
        <blockquote id="embed-fallback" class="wp-embedded-content" data-secret="abc123XYZ0">
          <a href="https://example.com/other-post/">Another post</a>
        </blockquote>
        <iframe id="embed" class="wp-embedded-content" sandbox="allow-scripts" data-secret="abc123XYZ0"
          width="500" height="282" style="width: 100%; max-width: 500px; height: auto; aspect-ratio: 500 / 282;"
          srcdoc="\(embedPage)"></iframe>
        <blockquote id="other-fallback" class="wp-embedded-content" data-secret="zzz999yyy8">
          <a href="https://example.com/third-post/">A third post</a>
        </blockquote>
        <div style="height: 1500px"></div>
        <p id="fn-1">The footnote.</p>
        <p><a name="named">A named anchor.</a></p>
        """

    /// A WordPress post's embed page: it sends its height when its parent says it's ready.
    static let embedPage = """
        <script>
          addEventListener('message', (event) => {
            if (event.data && event.data.message === 'ready') {
              parent.postMessage({ message: 'height', value: '345', secret: event.data.secret }, '*');
            }
          });
        </script>
        """
        .replacingOccurrences(of: "\"", with: "&quot;")

    static let checks = #"""
        const results = [];
        const check = (name, ok, detail = '') => {
          results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${ok ? '' : ' – ' + detail}`);
        };
        const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
        // Waits for `condition`, for up to `ms`, and says whether it held.
        const until = async (condition, ms = 5000) => {
          for (let waited = 0; !condition() && waited < ms; waited += 50) {
            await wait(50);
          }
          return condition();
        };
        const byId = (id) => document.getElementById(id);
        const style = (id) => getComputedStyle(byId(id));

        window.addEventListener('load', async () => {
          await wait(400);

          // Images
          const plain = byId('plain');
          check('the page gets the fade-in class', document.documentElement.classList.contains('reader-fades-images'));
          check('a loaded image is marked loaded', plain.classList.contains('reader-loaded'), plain.className);
          // Hidden pages don't run transitions, so skip to the end of the fade.
          plain.getAnimations().forEach((animation) => animation.finish());
          check('a loaded image ends up shown', style('plain').opacity === '1', style('plain').opacity);
          const broken = byId('broken');
          check(
            'a broken image stays hidden',
            !broken.classList.contains('reader-loaded') && style('broken').opacity === '0',
            style('broken').opacity
          );
          check(
            'an image becomes a link to itself',
            plain.parentElement.tagName === 'A' && plain.parentElement.href === plain.src,
            plain.parentElement.outerHTML.slice(0, 80)
          );
          const linked = byId('linked');
          check(
            'a linked image keeps its link',
            linked.parentElement.getAttribute('href') === 'https://example.com/post' &&
              linked.parentElement.parentElement.tagName !== 'A'
          );
          check('emoji aren\'t links', byId('emoji').parentElement.tagName === 'P');
          const pictured = byId('pictured');
          check(
            'a <picture> keeps its <img>, inside the link',
            pictured.parentElement.tagName === 'PICTURE' && pictured.parentElement.parentElement.tagName === 'A'
          );

          // In-page links
          messages.length = 0;
          byId('to-footnote').click();
          const top = Math.round(byId('fn-1').getBoundingClientRect().top + window.scrollY);
          check(
            'an in-page link tells the app the anchor and where it is',
            JSON.stringify(messages) === JSON.stringify([`anchor:${top}:fn-1`]),
            JSON.stringify(messages)
          );
          check('the in-page link doesn\'t navigate', location.hash === '', location.hash);
          messages.length = 0;
          byId('to-named').click();
          check(
            'an in-page link to a named anchor works',
            messages.length === 1 && /^anchor:\d+:named$/.test(messages[0]),
            JSON.stringify(messages)
          );
          messages.length = 0;
          byId('to-nowhere').click();
          check('a link to a missing place sends nothing', messages.length === 0, JSON.stringify(messages));
          check('a link to a missing place doesn\'t navigate', location.hash === '', location.hash);

          // Image taps
          const imageMessage = (id) => {
            const box = byId(id).getBoundingClientRect();
            const place = [box.left + scrollX, box.top + scrollY, box.width, box.height].map(Math.round);
            return `image:${place.join(':')}:${byId(id).getAttribute('src')}`;
          };
          const tap = (id) => {
            messages.length = 0;
            // Stops a link the script lets through from leaving the page.
            const stay = (event) => event.preventDefault();
            document.addEventListener('click', stay);
            byId(id).click();
            document.removeEventListener('click', stay);
            return JSON.stringify(messages);
          };
          check(
            'tapping an image that isn\'t a link tells the app which image, and where',
            tap('plain') === JSON.stringify([imageMessage('plain')]),
            JSON.stringify(messages)
          );
          check(
            'tapping an image linked to an image file does too',
            tap('file-linked') === JSON.stringify([imageMessage('file-linked')]),
            JSON.stringify(messages)
          );
          check('tapping an image linked to a page follows the link', tap('linked') === '[]', JSON.stringify(messages));
          check('tapping an emoji sends nothing', tap('emoji') === '[]', JSON.stringify(messages));

          // The page's height. The script hears about a change at the page's next rendering update.
          // A web view that isn't on screen only runs one every second or two, so give it time.
          const pageHeight = () => `height:${Math.ceil(document.documentElement.getBoundingClientRect().height)}`;
          const isReported = () => heights.at(-1) === pageHeight();
          check('the page reports its height', await until(isReported), `${heights.at(-1)} vs ${pageHeight()}`);
          const before = heights.length;
          const spacer = document.createElement('div');
          spacer.style.height = '300px';
          document.body.append(spacer);
          await until(isReported);
          check(
            'the page reports its height again when it changes',
            heights.length === before + 1 && heights.at(-1) === pageHeight(),
            `${JSON.stringify(heights.slice(before))} vs ${pageHeight()}`
          );
          spacer.remove();

          // Copying and selecting
          messages.length = 0;
          document.dispatchEvent(new Event('copy'));
          check('copying is reported', JSON.stringify(messages) === '["articleTextCopied"]', JSON.stringify(messages));
          messages.length = 0;
          getSelection().selectAllChildren(byId('text'));
          await wait(300);
          getSelection().selectAllChildren(byId('text'));
          await wait(1300);
          check(
            'selecting is reported once it settles',
            JSON.stringify(messages) === '["articleTextHighlighted"]',
            JSON.stringify(messages)
          );

          // WordPress post embeds
          check(
            'a WordPress embed asks for its height, and gets it',
            style('embed').height === '345px',
            style('embed').height
          );
          check('a WordPress embed hides its fallback', style('embed-fallback').display === 'none');
          check('another embed\'s fallback stays', style('other-fallback').display !== 'none');
          window.postMessage({ message: 'height', value: '900', secret: 'abc123XYZ0' }, '*');
          await wait(200);
          check(
            'a height from anywhere but the embed is ignored',
            style('embed').height === '345px',
            style('embed').height
          );

          byId('results').textContent = results.join('\n');
        });
        """#
}

/// Keeps the messages the page sends the app.
@MainActor
private final class MessageRecorder: NSObject, WKScriptMessageHandler {
    var messages: [String] = []

    func userContentController(_ userContentController: WKUserContentController, didReceive message: WKScriptMessage) {
        if let body = message.body as? String {
            messages.append(body)
        }
    }
}
#endif
