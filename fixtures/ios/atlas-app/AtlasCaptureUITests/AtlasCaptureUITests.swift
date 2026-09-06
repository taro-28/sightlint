import CryptoKit
import XCTest

private struct StringFact: Encodable {
    let sha256: String
    let utf8ByteLength: Int

    init?(_ value: String?) {
        guard let value, !value.isEmpty else { return nil }
        let data = Data(value.utf8)
        sha256 = "sha256:" + SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
        utf8ByteLength = data.count
    }
}

private struct PointRect: Encodable {
    let x: Double
    let y: Double
    let width: Double
    let height: Double

    init(_ rect: CGRect) {
        x = PointRect.round(rect.origin.x)
        y = PointRect.round(rect.origin.y)
        width = PointRect.round(rect.size.width)
        height = PointRect.round(rect.size.height)
    }

    private static func round(_ value: CGFloat) -> Double {
        let result = (Double(value) * 1000).rounded() / 1000
        return result == -0 ? 0 : result
    }
}

private struct XcuiNode: Encodable {
    let elementType: String
    let enabled: Bool
    let exists: Bool
    let focusStatus: String
    let framePoints: PointRect?
    let frameStatus: String
    let hittable: Bool
    let identifier: String
    let label: StringFact?
    let placeholder: StringFact?
    let query: String
    let selected: Bool
    let title: StringFact?
    let value: StringFact?
}

private struct XcuiHierarchy: Encodable {
    let nodes: [XcuiNode]
    let queryRoot: String
    let unmatchedQueryCount: Int
}

@MainActor
final class AtlasCaptureUITests: XCTestCase {
    override func setUpWithError() throws {
        continueAfterFailure = false
    }

    func testCaptureClean() throws {
        try capture(scenario: "clean")
    }

    func testCaptureOffCanvasControlMutant() throws {
        try capture(scenario: "off-canvas-control-mutant")
    }

    func testCaptureScrollOffscreenHardNegative() throws {
        try capture(scenario: "scroll-offscreen-hard-negative")
    }

    private func capture(scenario: String) throws {
        let app = XCUIApplication()
        app.launchEnvironment["SIGHTLINT_SCENARIO"] = scenario
        app.launchArguments += [
            "-AppleLanguages", "(en)",
            "-AppleLocale", "en_US",
            "-UIPreferredContentSizeCategoryName", "UICTContentSizeCategoryL",
            "-UIUserInterfaceStyle", "Light"
        ]
        app.launch()

        let ready = app.staticTexts["capture_ready"]
        XCTAssertTrue(ready.waitForExistence(timeout: 10))
        expectation(for: NSPredicate(format: "label == %@", "Capture ready"), evaluatedWith: ready)
        waitForExpectations(timeout: 10)

        // Capture pixels before querying individual XCUI elements. Evaluating an element outside
        // the visible scroll viewport can change UIScrollView state, so the screenshot must remain
        // paired with the source hierarchy written immediately before capture_ready is published.
        let screenshotAttachment = XCTAttachment(screenshot: app.screenshot())
        screenshotAttachment.name = "sightlint-screen-\(scenario).png"
        screenshotAttachment.lifetime = .keepAlways
        add(screenshotAttachment)

        var identifiers = [
            "account_subtitle",
            "account_title",
            "capture_ready",
            "notifications_detail",
            "notifications_label",
            "notifications_switch",
            "preferences_title",
            "profile_email",
            "profile_initials",
            "profile_name",
            "profile_plan",
            "save_button",
            "visibility_detail",
            "visibility_label",
            "visibility_value"
        ]
        if scenario == "scroll-offscreen-hard-negative" {
            identifiers += ["archived_detail", "archived_title"]
        }

        var unmatched = 0
        var nodes: [XcuiNode] = []
        for identifier in identifiers.sorted() {
            let query = "XCUIApplication.descendants(matching: .any)[\(identifier)]"
            let element = app.descendants(matching: .any)[identifier].firstMatch
            let exists = element.exists
            if !exists { unmatched += 1 }
            let frame = exists ? element.frame : .null
            let frameAvailable = exists && !frame.isNull && !frame.isInfinite
                && frame.width >= 0 && frame.height >= 0
            nodes.append(
                XcuiNode(
                    elementType: elementTypeName(element.elementType),
                    enabled: exists && element.isEnabled,
                    exists: exists,
                    focusStatus: "unavailable",
                    framePoints: frameAvailable ? PointRect(frame) : nil,
                    frameStatus: frameAvailable ? "exact" : "unavailable",
                    hittable: exists && element.isHittable,
                    identifier: identifier,
                    label: exists ? StringFact(element.label) : nil,
                    placeholder: nil,
                    query: query,
                    selected: exists && element.isSelected,
                    title: nil,
                    value: exists ? StringFact(element.value as? String) : nil
                )
            )
        }

        let hierarchy = XcuiHierarchy(
            nodes: nodes,
            queryRoot: "XCUIApplication",
            unmatchedQueryCount: unmatched
        )
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.sortedKeys, .withoutEscapingSlashes]
        let json = try encoder.encode(hierarchy)
        let jsonAttachment = XCTAttachment(data: json, uniformTypeIdentifier: "public.json")
        jsonAttachment.name = "sightlint-xcui-\(scenario).json"
        jsonAttachment.lifetime = .keepAlways
        add(jsonAttachment)

    }

    private func elementTypeName(_ type: XCUIElement.ElementType) -> String {
        switch type {
        case .application: "application"
        case .window: "window"
        case .other: "other"
        case .scrollView: "scrollView"
        case .staticText: "staticText"
        case .button: "button"
        case .switch: "switch"
        case .textField: "textField"
        default: "other"
        }
    }
}
