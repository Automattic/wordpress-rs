import Foundation
import SwiftUI
import WordPressAPI
import AuthenticationServices
import WordPressAPIInternal

struct AutoDiscoveryStepView: View {
    let step: AutoDiscoveryStep

    let successIcon = "checkmark.circle"
    let warningIcon = "exclamationmark.circle"
    let errorIcon   = "xmark.circle"

    var body: some View {
        VStack(alignment: .leading) {
            HStack(alignment: .firstTextBaseline) {
                if step.wasSuccessful {
                    Image(systemName: successIcon)
                        .font(.title)
                        .foregroundStyle(.green)
                } else if step.isRequired {
                    Image(systemName: errorIcon)
                        .font(.title)
                        .foregroundStyle(.red)
                } else {
                    Image(systemName: warningIcon)
                        .font(.title)
                        .foregroundStyle(.yellow)
                }

                Text(step.name).font(.title)

                Spacer()
            }

            if let errorMessage = step.errorMessage {
                Text(errorMessage)
            }

        }.padding(.horizontal)
    }
}

struct AutoDiscoveryErrorView: View {
    let errorMessage: String

    var body: some View {
        Text(errorMessage)
    }
}

struct AutodiscoveryReportView: View {
    let report: AutoDiscoveryResult?

    var body: some View {
        if let report {
            if let success = report.successfulAttempt {
                AutodiscoveryResultView(attempt: success)
            } else if let success = report.autoHttpsAttempt {
                AutodiscoveryResultView(attempt: success)
            } else {
                AutodiscoveryResultView(attempt: report.userInputAttempt)
            }
        } else {
            ProgressView()
        }
    }
}

struct AutodiscoveryResultView: View {
    let attempt: AutoDiscoveryAttemptResult

    let locale = Locale.autoupdatingCurrent

    var body: some View {
        if let url = attempt.domainWithSubdomain {
            Text(url)
        }

        let steps = attempt.constructSteps(langId: locale.identifier)

        ForEach(steps) { step in
            AutoDiscoveryStepView(step: step)
        }
    }
}

#Preview("Live data") {

    struct AsyncTestView: View {

        @State var report: AutoDiscoveryResult?

        private let loginApi = WordPressLoginClient(requestExecutor: URLSession.shared)

        var body: some View {
            AutodiscoveryReportView(report: report)
                .task {               
                    let result = await loginApi.autodiscoveryResult(forSite: "http://jalib923knblakis9ba92q3nbaslkes.nope")
                    self.report = result
                }
        }
    }

    return AsyncTestView()
}
