import Foundation
import Testing
import WordPressAPI

struct HTMLEntitiesTests {

    @Test
    func decodesAPostTitle() {
        let title = "WooCommerce for Android 3.5 is rolled out &#8211; 3.6 in code freeze"
        #expect(title.decodingHTMLEntities() == "WooCommerce for Android 3.5 is rolled out – 3.6 in code freeze")
    }

    @Test
    func decodesOnlyOnce() {
        #expect("&amp;amp;".decodingHTMLEntities() == "&amp;")
    }

    @Test
    func keepsTextThatIsNotAReference() {
        #expect("AT&T <em>Café</em> 😀".decodingHTMLEntities() == "AT&T <em>Café</em> 😀")
    }
}
