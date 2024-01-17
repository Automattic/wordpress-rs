package rs.wp_api

import org.junit.Test
import uniffi.wp_api.NetworkResponseStatus

class FakeNetworkResponseStatus: NetworkResponseStatus {
    override fun asU16(): UShort = 200u

    override fun isSuccess(): Boolean = true
    override fun isInformational(): Boolean = false
    override fun isRedirection(): Boolean = false
    override fun isClientError(): Boolean = false
    override fun isServerError(): Boolean = false
}

class NetworkResponseStatusTest {
    @Test
    fun networkResponseStatusCanBeOverridden() {
        val fakeStatus = FakeNetworkResponseStatus()
        assert(fakeStatus.asU16() == 200.toUShort())
        assert(fakeStatus.isSuccess())
        assert(!fakeStatus.isInformational())
        assert(!fakeStatus.isRedirection())
        assert(!fakeStatus.isClientError())
        assert(!fakeStatus.isServerError())
    }
}
