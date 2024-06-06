package rs.wordpress.example.shared.ui.welcome

import androidx.lifecycle.ViewModel
import rs.wordpress.example.shared.domain.AuthenticatedSite
import rs.wordpress.example.shared.repository.AuthenticationRepository

class WelcomeViewModel(private val authRepository: AuthenticationRepository): ViewModel() {
    fun getSiteList(): List<AuthenticatedSite> = authRepository.authenticatedSiteList()

    fun onSiteClicked(authenticatedSite: AuthenticatedSite) {}
}