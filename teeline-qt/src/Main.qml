import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs
import teeline_qt

pragma ComponentBehavior: Bound

ApplicationWindow {
    id: root
    visible: true
    width: 900
    height: 640
    title: "teeline-qt — TSP Solver"

    // ── Colour theme (single source of truth for all palette values) ────────
    QtObject {
        id: theme
        // Backgrounds
        readonly property color bgApp:       "#1a1a2e"  // main window
        readonly property color bgPanel:     "#13132b"  // headers / side panels
        readonly property color bgCard:      "#16213e"  // list items / stage cards
        readonly property color bgDeep:      "#0f0f23"  // visualization canvas
        readonly property color bgBar:       "#0d0d1e"  // bottom action bar
        readonly property color bgHighlight: "#1e3a5f"  // selected item / badges
        readonly property color bgDropHover: "#2a3f5f"  // drop-zone hover

        // Borders
        readonly property color border:        "#2a3a6a"
        readonly property color borderDrop:    "#3a4f7a"  // drop-zone outline
        readonly property color borderDiv:     "#2a2a4a"  // panel divider
        readonly property color overlayBorder: "#2a2a5a"  // KPI overlay
        readonly property color overlayBg:     "#cc0a0a1e" // KPI overlay (80 % alpha)

        // Accents
        readonly property color accent:      "#4fc3f7"  // primary blue
        readonly property color accentGreen: "#00e676"  // success / running

        // Text
        readonly property color textPrimary:  "#e0e0e0"
        readonly property color textSub:      "#b0b0b0"  // description body
        readonly property color textMuted:    "#9e9e9e"  // secondary labels
        readonly property color textLabel:    "#c8c8e0"  // form field labels
        readonly property color textDim:      "#607d8b"  // section headers
        readonly property color textHint:     "#7890a8"  // form hints
        readonly property color textDisabled: "#555555"  // disabled / empty states
        readonly property color textDark:     "#111122"  // on light backgrounds

        // Form fields
        readonly property color fieldBg:          "#dde0f5"
        readonly property color fieldBorder:      "#9090c0"
        readonly property color fieldPlaceholder: "#8888aa"
        readonly property color inputError:       "#c0302a"

        // Status
        readonly property color errorRed:   "#ef5350"
        readonly property color warnOrange: "#ff9800"
        readonly property color warnBorder: "#ff6d00"
        readonly property color warnBg:     "#3a1a00"
    }


    // ── Inline component: WelcomePage ───────────────────────────────────────
    component WelcomePage: Page {
        signal nextRequested()

        background: Rectangle { color: theme.bgApp }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 32
            spacing: 24

            Text {
                text: "teeline-qt"
                font.pixelSize: 28
                font.bold: true
                color: theme.textPrimary
            }

            // Drop zone
            Rectangle {
                Layout.fillWidth: true
                height: 160
                radius: 8
                color: dropArea.containsDrag ? theme.bgDropHover : theme.bgCard
                border.color: dropArea.containsDrag ? theme.accent : theme.borderDrop
                border.width: 2

                ColumnLayout {
                    anchors.centerIn: parent
                    spacing: 12
                    Text {
                        Layout.alignment: Qt.AlignHCenter
                        text: "Drop a .tsp file here"
                        font.pixelSize: 16
                        color: theme.textMuted
                    }
                    Button {
                        Layout.alignment: Qt.AlignHCenter
                        text: "Browse…"
                        onClicked: fileDialog.open()
                    }
                }

                DropArea {
                    id: dropArea
                    anchors.fill: parent
                    onDropped: function(drop) {
                        if (drop.urls.length > 0)
                            FileLoader.loadFile(drop.urls[0])
                    }
                }
            }

            // Error message
            Text {
                visible: FileLoader.errorMessage !== ""
                text: FileLoader.errorMessage
                color: theme.errorRed
                font.pixelSize: 13
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
            }

            // Metadata + mini-map
            RowLayout {
                visible: FileLoader.isLoaded
                Layout.fillWidth: true
                spacing: 24

                ColumnLayout {
                    spacing: 8
                    Layout.preferredWidth: 240
                    Text { text: "Problem:";       color: theme.textMuted; font.pixelSize: 12 }
                    Text { text: FileLoader.problemName;    color: theme.textPrimary; font.pixelSize: 14; font.bold: true }
                    Text { text: "Cities:";        color: theme.textMuted; font.pixelSize: 12 }
                    Text { text: FileLoader.cityCount;      color: theme.textPrimary; font.pixelSize: 14 }
                    Text { text: "Edge weights:";  color: theme.textMuted; font.pixelSize: 12 }
                    Text { text: FileLoader.edgeWeightType; color: theme.textPrimary; font.pixelSize: 14 }
                }

                Canvas {
                    id: miniMap
                    Layout.preferredWidth: 200
                    Layout.preferredHeight: 200

                    onPaint: {
                        var ctx = getContext("2d")
                        ctx.fillStyle = theme.bgDeep
                        ctx.beginPath()
                        ctx.roundedRect(0, 0, width, height, 4, 4)
                        ctx.fill()

                        var raw = FileLoader.citiesJson
                        if (!raw || raw === "[]") return
                        var cities = JSON.parse(raw)
                        var pad = 8, w = width - pad*2, h = height - pad*2
                        ctx.fillStyle = theme.accentGreen
                        for (var i = 0; i < cities.length; i++) {
                            ctx.beginPath()
                            ctx.arc(pad + cities[i].x * w, pad + cities[i].y * h, 2.5, 0, Math.PI*2)
                            ctx.fill()
                        }
                    }

                    Connections {
                        target: FileLoader
                        function onCitiesJsonChanged() { miniMap.requestPaint() }
                    }
                }
            }

            // Optimal tour row
            RowLayout {
                visible: FileLoader.isLoaded
                Layout.fillWidth: true
                spacing: 12

                Text { text: "Optimal tour:"; color: theme.textMuted; font.pixelSize: 12 }
                Text {
                    visible: FileLoader.hasOptTour
                    text: "✓  " + FileLoader.optTourFilePath.split("/").pop()
                          + "  (" + FileLoader.optTourCost.toFixed(2) + ")"
                    color: theme.accentGreen; font.pixelSize: 13
                }
                Text {
                    visible: !FileLoader.hasOptTour
                    text: "not loaded  (optional)"
                    color: theme.textMuted; font.pixelSize: 12; font.italic: true
                }
                Button {
                    text: FileLoader.hasOptTour ? "Replace…" : "Browse…"
                    onClicked: optTourDialog.open()
                }
            }

            // Recent files (persisted via Rust/FileLoader)
            ColumnLayout {
                id: recentSection
                property var recentList: FileLoader.recentFilesJson !== "[]"
                                         ? JSON.parse(FileLoader.recentFilesJson) : []
                visible: recentList.length > 0
                spacing: 4
                Text { text: "Recent files"; color: theme.textMuted; font.pixelSize: 12 }
                Repeater {
                    model: recentSection.recentList.slice(0, 5)
                    delegate: Text {
                        required property string modelData
                        text: modelData
                        color: theme.accent
                        font.pixelSize: 13
                        elide: Text.ElideLeft
                        Layout.preferredWidth: 420
                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: FileLoader.loadFile(modelData)
                        }
                    }
                }
            }

            Item { Layout.fillHeight: true }

            // Bottom bar
            RowLayout {
                Layout.fillWidth: true
                Item { Layout.fillWidth: true }
                Button {
                    text: "Next →"
                    enabled: FileLoader.isLoaded
                    highlighted: FileLoader.isLoaded
                    onClicked: nextRequested()
                }
            }
        }

        FileDialog {
            id: fileDialog
            title: "Open TSPLIB file"
            nameFilters: ["TSPLIB files (*.tsp)", "All files (*)"]
            onAccepted: FileLoader.loadFile(selectedFile)
        }

        FileDialog {
            id: optTourDialog
            title: "Open optimal tour file"
            nameFilters: ["Optimal tour files (*.opt.tour)", "All files (*)"]
            onAccepted: FileLoader.loadOptTour(selectedFile)
        }

    }

    // ── Solver metadata — populated from Rust backend at startup (issue #118) ──
    readonly property var solverList: JSON.parse(SolverEngine.solversJson)

    // ── Inline component: SolverPage ────────────────────────────────────────
    // ── Inline component: SolverPage (merged with ConfigPage, issues #104 + #106 + #114) ──
    component SolverPage: Page {
        id: solverPageRoot
        signal backRequested()
        signal solveRequested()
        signal pipelineRequested()

        background: Rectangle { color: theme.bgApp }

        Binding { target: SolverEngine; property: "optTourRouteJson"; value: FileLoader.optTourRouteJson }

        property int selectedIdx: -1
        property var selectedSolver: selectedIdx >= 0 ? root.solverList[selectedIdx] : null
        property bool exactWarning: selectedSolver !== null && selectedSolver.exact && FileLoader.cityCount > 20

        // ── Solver-option helpers (merged from ConfigPage) ─────────────────
        readonly property var saDefaults:  ({ epochs: 10000, cooling_rate: 0.0001, min_temperature: 0.001, max_temperature: 1000.0 })
        readonly property var gaDefaults:  ({ epochs: 10000, mutation_probability: 0.001, n_elite: 3 })
        readonly property var psoDefaults: ({ epochs: 10000, n_nearest: 30 })
        readonly property var csDefaults:  ({ epochs: 10000, mutation_probability: 0.001 })
        readonly property var fpaDefaults: ({ epochs: 10000, mutation_probability: 0.001 })

        property string selectedAlias: selectedSolver ? selectedSolver.alias : ""
        property bool isSA:  selectedAlias === "sa"  || selectedAlias === "simulated_annealing"
        property bool isGA:  selectedAlias === "ga"  || selectedAlias === "genetic_algorithm"
        property bool isPSO: selectedAlias === "pso" || selectedAlias === "particle_swarm"
        property bool isCS:  selectedAlias === "cs"  || selectedAlias === "cuckoo_search"
        property bool isFPA: selectedAlias === "fpa" || selectedAlias === "flower_pollination"

        // Reset fields to defaults whenever the selected solver changes
        onSelectedSolverChanged: {
            if      (isSA)  { epochsField.text = saDefaults.epochs.toString();  crField.text = saDefaults.cooling_rate.toString(); minTField.text = saDefaults.min_temperature.toString(); maxTField.text = saDefaults.max_temperature.toString() }
            else if (isGA)  { epochsField.text = gaDefaults.epochs.toString();  mpField.text = gaDefaults.mutation_probability.toString(); eliteField.text = gaDefaults.n_elite.toString() }
            else if (isPSO) { epochsField.text = psoDefaults.epochs.toString(); swarmField.text = psoDefaults.n_nearest.toString() }
            else if (isCS)  { epochsField.text = csDefaults.epochs.toString();  mpField2.text = csDefaults.mutation_probability.toString() }
            else if (isFPA) { epochsField.text = fpaDefaults.epochs.toString(); mpField2.text = fpaDefaults.mutation_probability.toString() }
        }

        function collectOpts() {
            if (!selectedSolver || !selectedSolver.hasOptions) return "{}"
            var obj = { epochs: parseInt(epochsField.text) || 10000 }
            if (isSA) {
                obj.cooling_rate    = parseFloat(crField.text)   || 0.0001
                obj.min_temperature = parseFloat(minTField.text) || 0.001
                obj.max_temperature = parseFloat(maxTField.text) || 1000.0
            } else if (isGA) {
                obj.mutation_probability = parseFloat(mpField.text) || 0.001
                obj.n_elite = parseInt(eliteField.text) || 3
            } else if (isPSO) {
                obj.n_nearest = parseInt(swarmField.text) || 30
            } else if (isCS || isFPA) {
                obj.mutation_probability = parseFloat(mpField2.text) || 0.001
            }
            return JSON.stringify(obj)
        }

        function hasError() {
            if (!selectedSolver || !selectedSolver.hasOptions) return false
            if (isNaN(parseInt(epochsField.text)) || parseInt(epochsField.text) < 1) return true
            if (isSA) {
                var cr = parseFloat(crField.text)
                if (isNaN(cr) || cr <= 0 || cr >= 1) return true
                var minT = parseFloat(minTField.text)
                var maxT = parseFloat(maxTField.text)
                if (isNaN(minT) || minT < 0) return true
                if (isNaN(maxT) || maxT <= 0) return true
                if (minT >= maxT) return true
            }
            if (isGA) {
                var mp = parseFloat(mpField.text)
                if (isNaN(mp) || mp < 0 || mp > 1) return true
                if (isNaN(parseInt(eliteField.text)) || parseInt(eliteField.text) < 1) return true
            }
            if (isCS || isFPA) {
                var mp2 = parseFloat(mpField2.text)
                if (isNaN(mp2) || mp2 < 0 || mp2 > 1) return true
            }
            if (isPSO) {
                if (isNaN(parseInt(swarmField.text)) || parseInt(swarmField.text) < 1) return true
            }
            return false
        }

        RowLayout {
            anchors.fill: parent
            spacing: 0

            // ── Left panel: solver list ──────────────────────────────────────
            Rectangle {
                Layout.preferredWidth: 280
                Layout.fillHeight: true
                color: theme.bgPanel

                ListView {
                    id: solverListView
                    anchors.fill: parent
                    anchors.margins: 8
                    model: root.solverList
                    clip: true

                    section.property: "category"
                    section.delegate: Rectangle {
                        width: solverListView.width
                        height: 28
                        color: "transparent"
                        required property string section
                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.left: parent.left
                            anchors.leftMargin: 8
                            text: section
                            font.pixelSize: 11
                            font.bold: true
                            color: theme.textDim
                            font.letterSpacing: 1.2
                        }
                    }

                    delegate: ItemDelegate {
                        width: solverListView.width
                        height: 36
                        required property var modelData
                        required property int index

                        property bool isExactWarning: modelData.exact && FileLoader.cityCount > 20

                        highlighted: index === solverPageRoot.selectedIdx
                        enabled: !isExactWarning

                        contentItem: Text {
                            text: modelData.name
                            color: isExactWarning ? theme.textDisabled : (index === solverPageRoot.selectedIdx ? theme.accentGreen : theme.textPrimary)
                            font.pixelSize: 13
                            verticalAlignment: Text.AlignVCenter
                        }

                        background: Rectangle {
                            color: index === solverPageRoot.selectedIdx ? theme.bgHighlight : "transparent"
                            radius: 4
                        }

                        onClicked: {
                            solverPageRoot.selectedIdx = index
                            SolverEngine.selectSolver(modelData.alias)
                        }

                        ToolTip.visible: isExactWarning && hovered
                        ToolTip.text: "Disabled: " + FileLoader.cityCount + " cities > 20 (exact solver)"
                        ToolTip.delay: 400
                    }
                }
            }

            // Divider
            Rectangle { width: 1; Layout.fillHeight: true; color: theme.borderDiv }

            // ── Right panel: description + config ────────────────────────────
            Item {
                Layout.fillWidth: true
                Layout.fillHeight: true

                // Scrollable content
                ScrollView {
                    id: rightScrollView
                    anchors { fill: parent; bottomMargin: 56 }
                    contentWidth: availableWidth
                    clip: true

                    ColumnLayout {
                        width: rightScrollView.availableWidth
                        spacing: 0

                        // Empty state
                        Text {
                            visible: selectedSolver === null
                            Layout.alignment: Qt.AlignHCenter
                            Layout.topMargin: 120
                            text: "Select a solver"
                            font.pixelSize: 18
                            color: theme.textDisabled
                        }

                        // Detail + config (when solver selected)
                        ColumnLayout {
                            visible: selectedSolver !== null
                            Layout.margins: 32
                            spacing: 12

                            // ── Description ───────────────────────────────────
                            Text {
                                text: selectedSolver ? selectedSolver.name : ""
                                font.pixelSize: 22; font.bold: true; color: theme.textPrimary
                            }
                            Text {
                                text: selectedSolver ? selectedSolver.category : ""
                                font.pixelSize: 12; color: theme.textDim; font.letterSpacing: 1
                            }
                            Text {
                                text: selectedSolver ? selectedSolver.desc : ""
                                font.pixelSize: 14; color: theme.textSub
                                wrapMode: Text.WordWrap; Layout.fillWidth: true
                            }
                            RowLayout {
                                spacing: 8
                                Text { text: "Complexity:"; color: theme.textDim; font.pixelSize: 12 }
                                Text {
                                    text: selectedSolver ? selectedSolver.complexity : ""
                                    color: theme.textPrimary; font.pixelSize: 12; font.family: "monospace"
                                }
                            }

                            // Exact solver warning
                            Rectangle {
                                visible: exactWarning
                                Layout.fillWidth: true
                                height: warnText.implicitHeight + 16
                                color: theme.warnBg; radius: 6
                                border.color: theme.warnBorder; border.width: 1
                                Text {
                                    id: warnText
                                    anchors { fill: parent; margins: 8 }
                                    text: "⚠  " + FileLoader.cityCount + " cities — exact solvers are practical only for n ≤ 20"
                                    color: theme.warnOrange; font.pixelSize: 12; wrapMode: Text.WordWrap
                                }
                            }

                            // ── Parameters section ────────────────────────────
                            Rectangle {
                                visible: selectedSolver !== null && selectedSolver.hasOptions
                                Layout.fillWidth: true
                                height: 1
                                color: theme.border
                                Layout.topMargin: 8
                            }
                            Text {
                                visible: selectedSolver !== null && selectedSolver.hasOptions
                                text: "PARAMETERS"
                                font.pixelSize: 11; font.bold: true
                                color: theme.textDim; font.letterSpacing: 1.2
                            }

                            // Epochs (all option-bearing solvers)
                            ColumnLayout {
                                visible: selectedSolver !== null && selectedSolver.hasOptions
                                spacing: 6; Layout.fillWidth: true
                                Text { text: "Epochs"; color: theme.textLabel; font.pixelSize: 13; font.bold: true }
                                TextField {
                                    id: epochsField
                                    Layout.preferredWidth: 220; font.pixelSize: 14
                                    text: "10000"; placeholderText: "10000"
                                    color: acceptableInput ? theme.textDark : theme.inputError
                                    placeholderTextColor: theme.fieldPlaceholder
                                    validator: IntValidator { bottom: 1; top: 10000000 }
                                    background: Rectangle { color: theme.fieldBg; radius: 4; border.color: parent.activeFocus ? theme.accent : theme.fieldBorder; border.width: 1 }
                                }
                                Text { text: "Number of iterations the solver will run"; color: theme.textHint; font.pixelSize: 11 }
                            }

                            // ── SA fields ─────────────────────────────────────
                            ColumnLayout {
                                visible: isSA
                                spacing: 16; Layout.fillWidth: true

                                ColumnLayout {
                                    spacing: 6; Layout.fillWidth: true
                                    Text { text: "Cooling rate"; color: theme.textLabel; font.pixelSize: 13; font.bold: true }
                                    TextField {
                                        id: crField
                                        Layout.preferredWidth: 220; font.pixelSize: 14
                                        text: saDefaults.cooling_rate.toString(); placeholderText: "0.0001"
                                        color: acceptableInput ? theme.textDark : theme.inputError
                                        placeholderTextColor: theme.fieldPlaceholder
                                        validator: DoubleValidator { bottom: 0.000001; top: 0.999999; notation: DoubleValidator.StandardNotation }
                                        background: Rectangle { color: theme.fieldBg; radius: 4; border.color: parent.activeFocus ? theme.accent : theme.fieldBorder; border.width: 1 }
                                    }
                                    Text { text: "Must be > 0 and < 1"; color: theme.textHint; font.pixelSize: 11 }
                                }

                                RowLayout {
                                    spacing: 32; Layout.fillWidth: true
                                    ColumnLayout {
                                        spacing: 6
                                        Text { text: "Min temperature"; color: theme.textLabel; font.pixelSize: 13; font.bold: true }
                                        TextField {
                                            id: minTField
                                            Layout.preferredWidth: 180; font.pixelSize: 14
                                            text: saDefaults.min_temperature.toString(); placeholderText: "0.001"
                                            color: theme.textDark; placeholderTextColor: theme.fieldPlaceholder
                                            validator: DoubleValidator { bottom: 0; notation: DoubleValidator.StandardNotation }
                                            background: Rectangle { color: theme.fieldBg; radius: 4; border.color: parent.activeFocus ? theme.accent : theme.fieldBorder; border.width: 1 }
                                        }
                                    }
                                    ColumnLayout {
                                        spacing: 6
                                        Text { text: "Max temperature"; color: theme.textLabel; font.pixelSize: 13; font.bold: true }
                                        TextField {
                                            id: maxTField
                                            Layout.preferredWidth: 180; font.pixelSize: 14
                                            text: saDefaults.max_temperature.toString(); placeholderText: "1000.0"
                                            color: theme.textDark; placeholderTextColor: theme.fieldPlaceholder
                                            validator: DoubleValidator { bottom: 0.000001; notation: DoubleValidator.StandardNotation }
                                            background: Rectangle { color: theme.fieldBg; radius: 4; border.color: parent.activeFocus ? theme.accent : theme.fieldBorder; border.width: 1 }
                                        }
                                    }
                                }
                                Text {
                                    visible: { var mn = parseFloat(minTField.text); var mx = parseFloat(maxTField.text); return !isNaN(mn) && !isNaN(mx) && mn >= mx }
                                    text: "⚠  Min temperature must be less than max temperature"
                                    color: theme.errorRed; font.pixelSize: 12
                                }
                            }

                            // ── GA fields ─────────────────────────────────────
                            ColumnLayout {
                                visible: isGA
                                spacing: 16; Layout.fillWidth: true

                                ColumnLayout {
                                    spacing: 6; Layout.fillWidth: true
                                    Text { text: "Mutation probability"; color: theme.textLabel; font.pixelSize: 13; font.bold: true }
                                    TextField {
                                        id: mpField
                                        Layout.preferredWidth: 220; font.pixelSize: 14
                                        text: gaDefaults.mutation_probability.toString(); placeholderText: "0.001"
                                        color: acceptableInput ? theme.textDark : theme.inputError
                                        placeholderTextColor: theme.fieldPlaceholder
                                        validator: DoubleValidator { bottom: 0; top: 1; notation: DoubleValidator.StandardNotation }
                                        background: Rectangle { color: theme.fieldBg; radius: 4; border.color: parent.activeFocus ? theme.accent : theme.fieldBorder; border.width: 1 }
                                    }
                                    Text { text: "Range [0, 1]"; color: theme.textHint; font.pixelSize: 11 }
                                }

                                ColumnLayout {
                                    spacing: 6
                                    Text { text: "Elite count"; color: theme.textLabel; font.pixelSize: 13; font.bold: true }
                                    TextField {
                                        id: eliteField
                                        Layout.preferredWidth: 140; font.pixelSize: 14
                                        text: gaDefaults.n_elite.toString(); placeholderText: "3"
                                        color: acceptableInput ? theme.textDark : theme.inputError
                                        placeholderTextColor: theme.fieldPlaceholder
                                        validator: IntValidator { bottom: 1; top: 1000 }
                                        background: Rectangle { color: theme.fieldBg; radius: 4; border.color: parent.activeFocus ? theme.accent : theme.fieldBorder; border.width: 1 }
                                    }
                                    Text { text: "Elite solutions preserved each generation"; color: theme.textHint; font.pixelSize: 11 }
                                }
                            }

                            // ── CS / FPA: mutation probability ─────────────────
                            ColumnLayout {
                                visible: isCS || isFPA
                                spacing: 6; Layout.fillWidth: true
                                Text { text: "Mutation probability"; color: theme.textLabel; font.pixelSize: 13; font.bold: true }
                                TextField {
                                    id: mpField2
                                    Layout.preferredWidth: 220; font.pixelSize: 14
                                    text: isCS ? csDefaults.mutation_probability.toString() : fpaDefaults.mutation_probability.toString()
                                    placeholderText: "0.001"
                                    color: acceptableInput ? theme.textDark : theme.inputError
                                    placeholderTextColor: theme.fieldPlaceholder
                                    validator: DoubleValidator { bottom: 0; top: 1; notation: DoubleValidator.StandardNotation }
                                    background: Rectangle { color: theme.fieldBg; radius: 4; border.color: parent.activeFocus ? theme.accent : theme.fieldBorder; border.width: 1 }
                                }
                                Text { text: "Range [0, 1]"; color: theme.textHint; font.pixelSize: 11 }
                            }

                            // ── PSO: swarm size ────────────────────────────────
                            ColumnLayout {
                                visible: isPSO
                                spacing: 6
                                Text { text: "Swarm size (n_nearest)"; color: theme.textLabel; font.pixelSize: 13; font.bold: true }
                                TextField {
                                    id: swarmField
                                    Layout.preferredWidth: 140; font.pixelSize: 14
                                    text: psoDefaults.n_nearest.toString(); placeholderText: "30"
                                    color: acceptableInput ? theme.textDark : theme.inputError
                                    placeholderTextColor: theme.fieldPlaceholder
                                    validator: IntValidator { bottom: 1; top: 10000 }
                                    background: Rectangle { color: theme.fieldBg; radius: 4; border.color: parent.activeFocus ? theme.accent : theme.fieldBorder; border.width: 1 }
                                }
                                Text { text: "Minimum particle count (floor is 30)"; color: theme.textHint; font.pixelSize: 11 }
                            }

                            Item { Layout.preferredHeight: 16 }
                        }
                    }
                }

                // ── Fixed bottom action bar ───────────────────────────────────
                Rectangle {
                    anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
                    height: 56
                    color: theme.bgBar
                    z: 10

                    RowLayout {
                        anchors { fill: parent; leftMargin: 20; rightMargin: 20 }
                        spacing: 12

                        Button {
                            text: "← Back"
                            onClicked: backRequested()
                        }

                        Button {
                            text: "Pipeline →"
                            onClicked: pipelineRequested()
                            contentItem: Text {
                                text: parent.text
                                color: theme.accent; font: parent.font
                                horizontalAlignment: Text.AlignHCenter
                                verticalAlignment: Text.AlignVCenter
                            }
                            ToolTip.visible: hovered
                            ToolTip.text: "Chain multiple solvers in sequence"
                            ToolTip.delay: 400
                        }

                        Item { Layout.fillWidth: true }

                        Text {
                            visible: selectedSolver !== null && selectedSolver.hasOptions && hasError()
                            text: "Fix validation errors above"
                            color: theme.errorRed; font.pixelSize: 12
                        }

                        Button {
                            text: "Solve ▶"
                            highlighted: true
                            visible: selectedSolver !== null
                            enabled: selectedSolver !== null && !exactWarning
                                     && (!selectedSolver.hasOptions || !hasError())
                            onClicked: {
                                if (selectedSolver.hasOptions)
                                    SolverEngine.startSolveWithOpts(FileLoader.filePath, collectOpts())
                                else
                                    SolverEngine.startSolve(FileLoader.filePath)
                                solveRequested()
                            }
                        }
                    }
                }
            }
        }
    }

    // ── Inline component: PipelinePage (issue #107) ─────────────────────────
    component PipelinePage: Page {
        id: pipelinePageRoot
        signal backRequested()
        signal solveRequested()

        background: Rectangle { color: theme.bgApp }

        // Each stage: { solver: "nn", name: "Nearest Neighbor", category: "Constructive" }
        property var stages: []

        function stagesJson() {
            return JSON.stringify(stages.map(function(s, i) {
                var item = stageRepeater.itemAt(i)
                return { solver: s.solver, opts: item ? item.getOpts() : {} }
            }))
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 0
            spacing: 0

            // ── Header ────────────────────────────────────────────────────
            Rectangle {
                Layout.fillWidth: true
                height: 60
                color: theme.bgPanel
                RowLayout {
                    anchors { fill: parent; leftMargin: 24; rightMargin: 24 }
                    Text { text: "Pipeline Builder"; font.pixelSize: 20; font.bold: true; color: theme.textPrimary }
                    Item { Layout.fillWidth: true }
                    Text {
                        text: stages.length + " stage" + (stages.length === 1 ? "" : "s")
                        color: theme.textDim; font.pixelSize: 13
                    }
                }
            }

            // ── Stage list ────────────────────────────────────────────────
            ScrollView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                contentWidth: availableWidth
                clip: true

                ColumnLayout {
                    width: parent.width
                    spacing: 8
                    Item { height: 8 }

                    // Empty state
                    ColumnLayout {
                        visible: stages.length === 0
                        Layout.fillWidth: true
                        Layout.topMargin: 60
                        spacing: 8
                        Text {
                            Layout.alignment: Qt.AlignHCenter
                            text: "No stages yet"
                            font.pixelSize: 18; color: theme.textDisabled
                        }
                        Text {
                            Layout.alignment: Qt.AlignHCenter
                            text: "Press + to add a solver stage"
                            font.pixelSize: 13; color: theme.textDisabled
                        }
                    }

                    // Stage cards
                    Repeater {
                        id: stageRepeater
                        model: stages
                        delegate: Rectangle {
                            required property var modelData
                            required property int index
                            width: parent ? parent.width - 48 : 400
                            radius: 6
                            color: theme.bgCard
                            border.color: configOpen ? theme.accent : theme.border
                            border.width: 1
                            clip: true

                            property bool configOpen: false
                            property bool hasSA:  modelData.solver === "sa"  || modelData.solver === "simulated_annealing"
                            property bool hasGA:  modelData.solver === "ga"  || modelData.solver === "genetic_algorithm"
                            property bool hasPSO: modelData.solver === "pso" || modelData.solver === "particle_swarm"
                            property bool hasCS:  modelData.solver === "cs"  || modelData.solver === "cuckoo_search"
                            property bool hasFPA: modelData.solver === "fpa" || modelData.solver === "flower_pollination"
                            property bool hasAnyOpts: hasSA || hasGA || hasPSO || hasCS || hasFPA

                            // Height: 64 base + accordion when open
                            property int accordionExpandedH: hasSA ? 240 : (hasGA ? 180 : 130)
                            height: 64 + (configOpen ? accordionExpandedH : 0)
                            Behavior on height { NumberAnimation { duration: 180; easing.type: Easing.InOutQuad } }

                            function getOpts() {
                                if (!hasAnyOpts) return {}
                                var opts = { epochs: parseInt(epochsField.text) || 10000 }
                                if (hasSA) {
                                    opts.cooling_rate    = parseFloat(crField.text)   || 0.0001
                                    opts.min_temperature = parseFloat(minTField.text) || 0.001
                                    opts.max_temperature = parseFloat(maxTField.text) || 1000.0
                                } else if (hasGA) {
                                    opts.mutation_probability = parseFloat(mpField.text) || 0.001
                                    opts.n_elite = parseInt(eliteField.text) || 3
                                } else if (hasPSO) {
                                    opts.n_nearest = parseInt(swarmField.text) || 30
                                } else if (hasCS || hasFPA) {
                                    opts.mutation_probability = parseFloat(mpField.text) || 0.001
                                }
                                return opts
                            }

                            // ── Main row ──────────────────────────────────────────────────
                            RowLayout {
                                id: mainRow
                                anchors { left: parent.left; right: parent.right; top: parent.top; leftMargin: 16; rightMargin: 8 }
                                height: 64
                                spacing: 8

                                // Stage number badge
                                Rectangle {
                                    width: 28; height: 28; radius: 14; color: theme.bgHighlight
                                    Text {
                                        anchors.centerIn: parent
                                        text: index + 1
                                        color: theme.accent; font.pixelSize: 13; font.bold: true
                                    }
                                }

                                // Arrow connector
                                Text {
                                    visible: index > 0
                                    text: "→"; color: theme.textDim; font.pixelSize: 14
                                }

                                ColumnLayout {
                                    spacing: 2; Layout.fillWidth: true
                                    Text { text: modelData.name;     color: theme.textPrimary; font.pixelSize: 14; font.bold: true }
                                    Text { text: modelData.category; color: theme.textDim; font.pixelSize: 11 }
                                }

                                // Config toggle
                                Button {
                                    text: configOpen ? "⚙ Close" : "⚙ Config"
                                    visible: hasAnyOpts
                                    contentItem: Text {
                                        text: parent.text
                                        color: configOpen ? theme.accent : theme.textDark
                                        font: parent.font
                                        horizontalAlignment: Text.AlignHCenter
                                        verticalAlignment: Text.AlignVCenter
                                    }
                                    ToolTip.visible: hovered
                                    ToolTip.text: configOpen ? "Close configuration" : "Configure solver options"
                                    ToolTip.delay: 400
                                    onClicked: configOpen = !configOpen
                                }

                                // Move up
                                Button {
                                    text: "↑ Up"
                                    enabled: index > 0
                                    opacity: index > 0 ? 1.0 : 0.3
                                    ToolTip.visible: hovered
                                    ToolTip.text: "Move stage earlier in pipeline"
                                    ToolTip.delay: 400
                                    onClicked: {
                                        var arr = pipelinePageRoot.stages.slice()
                                        var tmp = arr[index - 1]
                                        arr[index - 1] = arr[index]
                                        arr[index] = tmp
                                        pipelinePageRoot.stages = arr
                                    }
                                }

                                // Move down
                                Button {
                                    text: "↓ Down"
                                    enabled: index < pipelinePageRoot.stages.length - 1
                                    opacity: index < pipelinePageRoot.stages.length - 1 ? 1.0 : 0.3
                                    ToolTip.visible: hovered
                                    ToolTip.text: "Move stage later in pipeline"
                                    ToolTip.delay: 400
                                    onClicked: {
                                        var arr = pipelinePageRoot.stages.slice()
                                        var tmp = arr[index + 1]
                                        arr[index + 1] = arr[index]
                                        arr[index] = tmp
                                        pipelinePageRoot.stages = arr
                                    }
                                }

                                // Remove
                                Button {
                                    text: "✕ Remove"
                                    ToolTip.visible: hovered
                                    ToolTip.text: "Remove this stage"
                                    ToolTip.delay: 400
                                    contentItem: Text {
                                        text: parent.text
                                        color: theme.errorRed
                                        font: parent.font
                                        horizontalAlignment: Text.AlignHCenter
                                        verticalAlignment: Text.AlignVCenter
                                    }
                                    onClicked: {
                                        var arr = pipelinePageRoot.stages.slice()
                                        arr.splice(index, 1)
                                        pipelinePageRoot.stages = arr
                                    }
                                }
                            }

                            // ── Accordion separator ───────────────────────────────────────
                            Rectangle {
                                anchors { left: parent.left; right: parent.right; top: mainRow.bottom }
                                height: 1
                                color: theme.border
                            }

                            // ── Accordion: solver options ─────────────────────────────────
                            ColumnLayout {
                                id: accordionContent
                                anchors { left: parent.left; right: parent.right; top: mainRow.bottom; topMargin: 10; leftMargin: 20; rightMargin: 20 }
                                spacing: 10

                                // Epochs (all configurable solvers)
                                RowLayout {
                                    spacing: 12
                                    Text { text: "Epochs"; color: theme.textMuted; font.pixelSize: 12; Layout.preferredWidth: 148 }
                                    TextField {
                                        id: epochsField
                                        Layout.preferredWidth: 180
                                        text: "10000"; color: theme.textDark; font.pixelSize: 13
                                        validator: IntValidator { bottom: 1; top: 10000000 }
                                        background: Rectangle { color: theme.fieldBg; radius: 4; border.color: parent.activeFocus ? theme.accent : theme.fieldBorder; border.width: 1 }
                                    }
                                }

                                // SA: cooling rate
                                RowLayout {
                                    visible: hasSA; spacing: 12
                                    Text { text: "Cooling rate"; color: theme.textMuted; font.pixelSize: 12; Layout.preferredWidth: 148 }
                                    TextField {
                                        id: crField
                                        Layout.preferredWidth: 180
                                        text: "0.0001"; color: theme.textDark; font.pixelSize: 13
                                        background: Rectangle { color: theme.fieldBg; radius: 4; border.color: parent.activeFocus ? theme.accent : theme.fieldBorder; border.width: 1 }
                                    }
                                }

                                // SA: min temperature
                                RowLayout {
                                    visible: hasSA; spacing: 12
                                    Text { text: "Min temperature"; color: theme.textMuted; font.pixelSize: 12; Layout.preferredWidth: 148 }
                                    TextField {
                                        id: minTField
                                        Layout.preferredWidth: 180
                                        text: "0.001"; color: theme.textDark; font.pixelSize: 13
                                        background: Rectangle { color: theme.fieldBg; radius: 4; border.color: parent.activeFocus ? theme.accent : theme.fieldBorder; border.width: 1 }
                                    }
                                }

                                // SA: max temperature
                                RowLayout {
                                    visible: hasSA; spacing: 12
                                    Text { text: "Max temperature"; color: theme.textMuted; font.pixelSize: 12; Layout.preferredWidth: 148 }
                                    TextField {
                                        id: maxTField
                                        Layout.preferredWidth: 180
                                        text: "1000.0"; color: theme.textDark; font.pixelSize: 13
                                        background: Rectangle { color: theme.fieldBg; radius: 4; border.color: parent.activeFocus ? theme.accent : theme.fieldBorder; border.width: 1 }
                                    }
                                }

                                // GA / CS / FPA: mutation probability
                                RowLayout {
                                    visible: hasGA || hasCS || hasFPA; spacing: 12
                                    Text { text: "Mutation probability"; color: theme.textMuted; font.pixelSize: 12; Layout.preferredWidth: 148 }
                                    TextField {
                                        id: mpField
                                        Layout.preferredWidth: 180
                                        text: "0.001"; color: theme.textDark; font.pixelSize: 13
                                        background: Rectangle { color: theme.fieldBg; radius: 4; border.color: parent.activeFocus ? theme.accent : theme.fieldBorder; border.width: 1 }
                                    }
                                }

                                // GA: elite count
                                RowLayout {
                                    visible: hasGA; spacing: 12
                                    Text { text: "Elite count"; color: theme.textMuted; font.pixelSize: 12; Layout.preferredWidth: 148 }
                                    TextField {
                                        id: eliteField
                                        Layout.preferredWidth: 180
                                        text: "3"; color: theme.textDark; font.pixelSize: 13
                                        validator: IntValidator { bottom: 1; top: 1000 }
                                        background: Rectangle { color: theme.fieldBg; radius: 4; border.color: parent.activeFocus ? theme.accent : theme.fieldBorder; border.width: 1 }
                                    }
                                }

                                // PSO: swarm size (n_nearest)
                                RowLayout {
                                    visible: hasPSO; spacing: 12
                                    Text { text: "Swarm size (n_nearest)"; color: theme.textMuted; font.pixelSize: 12; Layout.preferredWidth: 148 }
                                    TextField {
                                        id: swarmField
                                        Layout.preferredWidth: 180
                                        text: "30"; color: theme.textDark; font.pixelSize: 13
                                        validator: IntValidator { bottom: 1; top: 10000 }
                                        background: Rectangle { color: theme.fieldBg; radius: 4; border.color: parent.activeFocus ? theme.accent : theme.fieldBorder; border.width: 1 }
                                    }
                                }
                            }
                        }
                    }

                    // Add stage button
                    Button {
                        Layout.alignment: Qt.AlignHCenter
                        Layout.topMargin: 8
                        text: "+ Add Stage"
                        onClicked: addStagePopup.open()
                    }
                }
            }

            // ── Bottom bar ─────────────────────────────────────────────────
            Rectangle {
                Layout.fillWidth: true
                height: 56
                color: theme.bgBar

                RowLayout {
                    anchors { fill: parent; leftMargin: 20; rightMargin: 20 }
                    spacing: 12

                    Button { text: "← Back"; onClicked: backRequested() }

                    Item { Layout.fillWidth: true }

                    Text {
                        visible: stages.length < 2
                        text: "Add at least 2 stages to run"
                        color: theme.textDim; font.pixelSize: 12
                    }

                    Button {
                        text: "Run Pipeline ▶"
                        highlighted: true
                        enabled: stages.length >= 1
                        onClicked: {
                            SolverEngine.runPipelineStages(FileLoader.filePath, stagesJson())
                            solveRequested()
                        }
                    }
                }
            }
        }

        // ── Add-stage popup ───────────────────────────────────────────────
        Popup {
            id: addStagePopup
            parent: Overlay.overlay
            anchors.centerIn: parent
            width: 340
            height: Math.min(480, solverListView2.contentHeight + 80)
            modal: true
            closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

            background: Rectangle { color: theme.bgPanel; radius: 8; border.color: theme.border; border.width: 1 }

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 16
                spacing: 8

                Text { text: "Add a stage"; font.pixelSize: 16; font.bold: true; color: theme.textPrimary }

                ListView {
                    id: solverListView2
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    model: root.solverList
                    clip: true
                    spacing: 2

                    delegate: ItemDelegate {
                        id: addStageDelegateItem
                        width: solverListView2.width
                        height: 40
                        required property var modelData

                        contentItem: ColumnLayout {
                            spacing: 1
                            Text { text: modelData.name; color: theme.textPrimary; font.pixelSize: 13 }
                            Text { text: modelData.category; color: theme.textDim; font.pixelSize: 10 }
                        }

                        background: Rectangle {
                            color: addStageDelegateItem.hovered ? theme.bgHighlight : "transparent"; radius: 4
                        }

                        onClicked: {
                            var arr = pipelinePageRoot.stages.slice()
                            arr.push({ solver: modelData.alias, name: modelData.name, category: modelData.category })
                            pipelinePageRoot.stages = arr
                            addStagePopup.close()
                        }
                    }
                }
            }
        }
    }

    // ── Inline component: VisualizationPage (issue #105) ────────────────────
    component VisualizationPage: Page {
        signal backRequested()

        background: Rectangle { color: theme.bgDeep }

        property var compData: SolverEngine.comparisonJson.length > 0 ? JSON.parse(SolverEngine.comparisonJson) : null

        // ── Tour canvas ────────────────────────────────────────────────────
        Canvas {
            id: tourCanvas
            anchors.fill: parent

            property var cityPos: ({})   // id → {x,y} in canvas coords

            function rebuild() {
                var raw = FileLoader.citiesJson
                if (!raw || raw === "[]") { cityPos = {}; return }
                var cities = JSON.parse(raw)
                var pad = 48, w = width - pad*2, h = height - pad*2 - 56
                var m = {}
                for (var i = 0; i < cities.length; i++) {
                    m[cities[i].id] = {
                        x: pad + cities[i].x * w,
                        y: pad + cities[i].y * h
                    }
                }
                cityPos = m
            }

            onWidthChanged:  { rebuild(); requestPaint() }
            onHeightChanged: { rebuild(); requestPaint() }
            Component.onCompleted: { rebuild(); requestPaint() }

            onPaint: {
                var ctx = getContext("2d")
                ctx.fillStyle = theme.bgDeep
                ctx.fillRect(0, 0, width, height)

                var pos = cityPos
                var ids = Object.keys(pos)
                if (!ids.length) return

                // Tour edges
                var tourRaw = SolverEngine.tourJson
                if (tourRaw && tourRaw !== "[]") {
                    var tour = JSON.parse(tourRaw)
                    if (tour.length > 1) {
                        ctx.strokeStyle = theme.accentGreen
                        ctx.lineWidth = 1.2
                        ctx.globalAlpha = 0.65
                        ctx.beginPath()
                        var p0 = pos[tour[0]]
                        if (p0) { ctx.moveTo(p0.x, p0.y) }
                        for (var j = 1; j < tour.length; j++) {
                            var p = pos[tour[j]]
                            if (p) ctx.lineTo(p.x, p.y)
                        }
                        if (p0) ctx.lineTo(p0.x, p0.y)
                        ctx.stroke()
                        ctx.globalAlpha = 1.0
                    }
                }

                // Optimal tour overlay (dashed gray)
                if (FileLoader.hasOptTour) {
                    var optRaw = FileLoader.optTourRouteJson
                    if (optRaw && optRaw !== "[]") {
                        var optTour = JSON.parse(optRaw)
                        if (optTour.length > 1) {
                            ctx.strokeStyle = "#888888"
                            ctx.lineWidth = 1.0
                            ctx.globalAlpha = 0.45
                            ctx.setLineDash([6, 4])
                            ctx.beginPath()
                            var op0 = pos[optTour[0]]
                            if (op0) ctx.moveTo(op0.x, op0.y)
                            for (var k = 1; k < optTour.length; k++) {
                                var op = pos[optTour[k]]
                                if (op) ctx.lineTo(op.x, op.y)
                            }
                            if (op0) ctx.lineTo(op0.x, op0.y)
                            ctx.stroke()
                            ctx.setLineDash([])
                            ctx.globalAlpha = 1.0
                        }
                    }
                }

                // City dots
                ctx.fillStyle = theme.accent
                for (var i = 0; i < ids.length; i++) {
                    var c = pos[ids[i]]
                    ctx.beginPath()
                    ctx.arc(c.x, c.y, 3.5, 0, Math.PI*2)
                    ctx.fill()
                }
            }

            Connections {
                target: SolverEngine
                function onTourJsonChanged() { tourCanvas.requestPaint() }
            }
            Connections {
                target: FileLoader
                function onCitiesJsonChanged() { tourCanvas.rebuild(); tourCanvas.requestPaint() }
                function onOptTourRouteJsonChanged() { tourCanvas.requestPaint() }
            }
        }

        // ── KPI overlay (top-right) ────────────────────────────────────────
        Rectangle {
            anchors.top: parent.top
            anchors.right: parent.right
            anchors.margins: 12
            width: kpiCol.implicitWidth + 24
            height: kpiCol.implicitHeight + 20
            color: theme.overlayBg
            radius: 8
            border.color: theme.overlayBorder
            border.width: 1
            z: 10

            ColumnLayout {
                id: kpiCol
                anchors.centerIn: parent
                spacing: 6

                RowLayout {
                    spacing: 12
                    Text { text: "Solver"; color: theme.textDim; font.pixelSize: 11 }
                    Text { text: SolverEngine.selectedSolver; color: theme.accent; font.pixelSize: 13; font.bold: true }
                }
                RowLayout {
                    spacing: 12
                    Text { text: "Best cost"; color: theme.textDim; font.pixelSize: 11 }
                    Text {
                        text: SolverEngine.bestCost > 0 ? SolverEngine.bestCost.toFixed(2) : "—"
                        color: theme.accentGreen; font.pixelSize: 15; font.bold: true
                    }
                }
                RowLayout {
                    spacing: 12
                    Text { text: "Iteration"; color: theme.textDim; font.pixelSize: 11 }
                    Text { text: SolverEngine.iteration; color: theme.textPrimary; font.pixelSize: 13 }
                }
                RowLayout {
                    spacing: 12
                    Text { text: "Elapsed"; color: theme.textDim; font.pixelSize: 11 }
                    Text {
                        text: {
                            var ms = SolverEngine.elapsedMs
                            if (ms < 1000) return ms + " ms"
                            return (ms / 1000).toFixed(1) + " s"
                        }
                        color: theme.textPrimary; font.pixelSize: 13
                    }
                }
                RowLayout {
                    visible: compData !== null && !SolverEngine.running
                    spacing: 12
                    Text { text: "Optimal"; color: theme.textDim; font.pixelSize: 11 }
                    Text {
                        text: compData ? compData.optimalCost.toFixed(2) : "—"
                        color: theme.textMuted; font.pixelSize: 13
                    }
                }
                RowLayout {
                    visible: compData !== null && !SolverEngine.running
                    spacing: 12
                    Text { text: "Gap"; color: theme.textDim; font.pixelSize: 11 }
                    Text {
                        text: compData ? "+" + compData.gapPct.toFixed(2) + "%" : "—"
                        color: theme.warnOrange; font.pixelSize: 14; font.bold: true
                    }
                }

                // Running indicator
                RowLayout {
                    visible: SolverEngine.running
                    spacing: 6
                    Rectangle {
                        Layout.preferredWidth: 8; Layout.preferredHeight: 8; radius: 4; color: theme.accentGreen
                        SequentialAnimation on opacity {
                            loops: Animation.Infinite
                            NumberAnimation { to: 0.2; duration: 600 }
                            NumberAnimation { to: 1.0; duration: 600 }
                        }
                    }
                    Text { text: "Solving…"; color: theme.accentGreen; font.pixelSize: 11 }
                }
            }
        }

        // ── Bottom bar ─────────────────────────────────────────────────────
        Rectangle {
            anchors.bottom: parent.bottom
            anchors.left: parent.left
            anchors.right: parent.right
            height: 56
            color: theme.bgBar
            z: 10

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 20
                anchors.rightMargin: 20
                spacing: 12

                Button {
                    text: "← Back"
                    visible: !SolverEngine.running
                    onClicked: backRequested()
                }

                Button {
                    text: "Stop"
                    visible: SolverEngine.running
                    onClicked: SolverEngine.cancel()
                }

                Item { Layout.fillWidth: true }

                Text {
                    visible: !SolverEngine.running && SolverEngine.bestCost > 0
                    text: "Tour length: " + SolverEngine.bestCost.toFixed(2)
                    color: theme.accentGreen
                    font.pixelSize: 15
                    font.bold: true
                }

                Text {
                    visible: !SolverEngine.running && compData !== null
                    text: "Gap: +" + (compData ? compData.gapPct.toFixed(2) : "0.00") + "%"
                    color: theme.warnOrange
                    font.pixelSize: 13
                }

                Button {
                    text: "← New problem"
                    visible: !SolverEngine.running && SolverEngine.bestCost > 0
                    onClicked: backRequested()
                }
            }
        }
    }

    // ── StackView navigation ────────────────────────────────────────────────
    StackView {
        id: stackView
        anchors.fill: parent
        initialItem: welcomeComp
    }

    Component {
        id: welcomeComp
        WelcomePage {
            onNextRequested: stackView.push(solverComp)
        }
    }

    Component {
        id: pipelineComp
        PipelinePage {
            onBackRequested:  stackView.pop()
            onSolveRequested: stackView.push(vizComp)
        }
    }

    Component {
        id: solverComp
        SolverPage {
            onBackRequested:     stackView.pop()
            onPipelineRequested: stackView.push(pipelineComp)
            onSolveRequested:    stackView.push(vizComp)
        }
    }

    Component {
        id: vizComp
        VisualizationPage { onBackRequested: stackView.pop() }
    }
}
