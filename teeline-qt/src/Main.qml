import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs
import teeline_qt

ApplicationWindow {
    id: root
    visible: true
    width: 900
    height: 640
    title: "teeline-qt — TSP Solver"

    // ── Inline component: WelcomePage ───────────────────────────────────────
    component WelcomePage: Page {
        signal nextRequested()
        signal pipelineRequested()

        background: Rectangle { color: "#1a1a2e" }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 32
            spacing: 24

            Text {
                text: "teeline-qt"
                font.pixelSize: 28
                font.bold: true
                color: "#e0e0e0"
            }

            // Drop zone
            Rectangle {
                Layout.fillWidth: true
                height: 160
                radius: 8
                color: dropArea.containsDrag ? "#2a3f5f" : "#16213e"
                border.color: dropArea.containsDrag ? "#4fc3f7" : "#3a4f7a"
                border.width: 2

                ColumnLayout {
                    anchors.centerIn: parent
                    spacing: 12
                    Text {
                        Layout.alignment: Qt.AlignHCenter
                        text: "Drop a .tsp file here"
                        font.pixelSize: 16
                        color: "#9e9e9e"
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
                            FileLoader.loadFile(drop.urls[0].toString())
                    }
                }
            }

            // Error message
            Text {
                visible: FileLoader.errorMessage !== ""
                text: FileLoader.errorMessage
                color: "#ef5350"
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
                    Text { text: "Problem:";       color: "#9e9e9e"; font.pixelSize: 12 }
                    Text { text: FileLoader.problemName;    color: "#e0e0e0"; font.pixelSize: 14; font.bold: true }
                    Text { text: "Cities:";        color: "#9e9e9e"; font.pixelSize: 12 }
                    Text { text: FileLoader.cityCount;      color: "#e0e0e0"; font.pixelSize: 14 }
                    Text { text: "Edge weights:";  color: "#9e9e9e"; font.pixelSize: 12 }
                    Text { text: FileLoader.edgeWeightType; color: "#e0e0e0"; font.pixelSize: 14 }
                }

                Canvas {
                    id: miniMap
                    width: 200
                    height: 200

                    onPaint: {
                        var ctx = getContext("2d")
                        ctx.fillStyle = "#0f0f23"
                        ctx.beginPath()
                        ctx.roundedRect(0, 0, width, height, 4, 4)
                        ctx.fill()

                        var raw = FileLoader.citiesJson
                        if (!raw || raw === "[]") return
                        var cities = JSON.parse(raw)
                        var pad = 8, w = width - pad*2, h = height - pad*2
                        ctx.fillStyle = "#00e676"
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

            // Recent files (persisted via Rust/FileLoader)
            ColumnLayout {
                id: recentSection
                property var recentList: FileLoader.recentFilesJson !== "[]"
                                         ? JSON.parse(FileLoader.recentFilesJson) : []
                visible: recentList.length > 0
                spacing: 4
                Text { text: "Recent files"; color: "#9e9e9e"; font.pixelSize: 12 }
                Repeater {
                    model: recentSection.recentList.slice(0, 5)
                    delegate: Text {
                        required property string modelData
                        text: modelData
                        color: "#4fc3f7"
                        font.pixelSize: 13
                        elide: Text.ElideLeft
                        width: 420
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
                Switch {
                    id: advancedSwitch
                    text: "Pipeline Builder"
                    enabled: FileLoader.isLoaded
                    opacity: FileLoader.isLoaded ? 1.0 : 0.4
                }
                Item { Layout.fillWidth: true }
                Button {
                    text: "Next →"
                    enabled: FileLoader.isLoaded
                    highlighted: FileLoader.isLoaded
                    onClicked: advancedSwitch.checked ? pipelineRequested() : nextRequested()
                }
            }
        }

        FileDialog {
            id: fileDialog
            title: "Open TSPLIB file"
            nameFilters: ["TSPLIB files (*.tsp)", "All files (*)"]
            onAccepted: FileLoader.loadFile(selectedFile.toString())
        }

    }

    // ── Solver metadata (static) ────────────────────────────────────────────
    readonly property var solverList: [
        { name: "Bellman-Held-Karp",    alias: "bhk",            category: "Exact",
          desc: "Exact dynamic-programming solution. Optimal tour guaranteed.",
          complexity: "O(n² · 2ⁿ)", hasOptions: false, exact: true },
        { name: "Branch & Bound",       alias: "branch_bound",   category: "Exact",
          desc: "Exact branch-and-bound with lower-bound pruning.",
          complexity: "O(n!)",        hasOptions: false, exact: true },
        { name: "Nearest Neighbor",     alias: "nn",             category: "Constructive",
          desc: "Greedy heuristic: always visit the nearest unvisited city.",
          complexity: "O(n²)",        hasOptions: false, exact: false },
        { name: "2-opt",                alias: "2opt",           category: "Local Search",
          desc: "Iteratively reverses sub-tours to remove crossing edges.",
          complexity: "O(n²) / pass", hasOptions: false, exact: false },
        { name: "3-opt",                alias: "3opt",           category: "Local Search",
          desc: "Extends 2-opt by considering triple-edge reconnections.",
          complexity: "O(n³) / pass", hasOptions: false, exact: false },
        { name: "Simulated Annealing",  alias: "sa",             category: "Metaheuristic",
          desc: "Accepts worse moves with decreasing probability to escape local optima.",
          complexity: "O(epochs · n)", hasOptions: true,  exact: false },
        { name: "Genetic Algorithm",    alias: "ga",             category: "Metaheuristic",
          desc: "Evolves a population of tours via crossover and mutation operators.",
          complexity: "O(epochs · pop · n)", hasOptions: true, exact: false },
        { name: "Particle Swarm",       alias: "pso",            category: "Metaheuristic",
          desc: "Discrete PSO with velocity-capped particles guided by a global best.",
          complexity: "O(epochs · swarm · n)", hasOptions: true, exact: false },
        { name: "Cuckoo Search",        alias: "cs",             category: "Metaheuristic",
          desc: "Lévy-flight search with probabilistic nest abandonment.",
          complexity: "O(epochs · nests · n)", hasOptions: true, exact: false },
        { name: "Flower Pollination",   alias: "fpa",            category: "Metaheuristic",
          desc: "Global Lévy-flight toward best tour; local ε-scaled cross-pollination.",
          complexity: "O(epochs · pop · n)", hasOptions: true, exact: false },
        { name: "Stochastic Hill Climb",alias: "stochastic_hill",category: "Metaheuristic",
          desc: "Random-restart hill climbing to escape local optima.",
          complexity: "O(epochs · n)", hasOptions: false, exact: false },
        { name: "Tabu Search",          alias: "tabu_search",    category: "Metaheuristic",
          desc: "Local search with a memory structure to avoid revisiting solutions.",
          complexity: "O(epochs · n)", hasOptions: false, exact: false },
        { name: "Random Shuffle",       alias: "shuffle",        category: "Utility",
          desc: "Baseline random tour. Useful as a warm-start seed for pipelines.",
          complexity: "O(n)",          hasOptions: false, exact: false },
    ]

    // ── Inline component: SolverPage ────────────────────────────────────────
    component SolverPage: Page {
        signal backRequested()
        signal solveRequested()
        signal configureRequested()

        background: Rectangle { color: "#1a1a2e" }

        // Track which item is selected by index
        property int selectedIdx: -1
        property var selectedSolver: selectedIdx >= 0 ? solverList[selectedIdx] : null

        // Disable exact solvers when problem is too large
        property bool exactWarning: selectedSolver !== null
                                    && selectedSolver.exact
                                    && FileLoader.cityCount > 20

        RowLayout {
            anchors.fill: parent
            anchors.margins: 0
            spacing: 0

            // ── Left panel: solver list ──────────────────────────────────────
            Rectangle {
                Layout.preferredWidth: 280
                Layout.fillHeight: true
                color: "#13132b"

                ListView {
                    id: solverListView
                    anchors.fill: parent
                    anchors.margins: 8
                    model: solverList
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
                            color: "#607d8b"
                            font.letterSpacing: 1.2
                        }
                    }

                    delegate: ItemDelegate {
                        width: solverListView.width
                        height: 36
                        required property var modelData
                        required property int index

                        property bool isExactWarning: modelData.exact && FileLoader.cityCount > 20

                        highlighted: index === selectedIdx
                        enabled: !isExactWarning

                        contentItem: Text {
                            text: modelData.name
                            color: isExactWarning ? "#555" : (index === selectedIdx ? "#00e676" : "#e0e0e0")
                            font.pixelSize: 13
                            verticalAlignment: Text.AlignVCenter
                        }

                        background: Rectangle {
                            color: index === selectedIdx ? "#1e3a5f" : "transparent"
                            radius: 4
                        }

                        onClicked: {
                            selectedIdx = index
                            SolverEngine.selectSolver(modelData.alias)
                        }

                        ToolTip.visible: isExactWarning && hovered
                        ToolTip.text: "Disabled: " + FileLoader.cityCount + " cities > 20 (exact solver)"
                        ToolTip.delay: 400
                    }
                }
            }

            // Divider
            Rectangle { width: 1; Layout.fillHeight: true; color: "#2a2a4a" }

            // ── Right panel: detail ──────────────────────────────────────────
            ColumnLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.margins: 32
                spacing: 16

                // No selection state
                ColumnLayout {
                    visible: selectedSolver === null
                    Layout.fillWidth: true
                    spacing: 8
                    Item { Layout.fillHeight: true }
                    Text {
                        Layout.alignment: Qt.AlignHCenter
                        text: "Select a solver"
                        font.pixelSize: 18
                        color: "#555"
                    }
                    Item { Layout.fillHeight: true }
                }

                // Detail panel
                ColumnLayout {
                    visible: selectedSolver !== null
                    Layout.fillWidth: true
                    spacing: 12

                    Text {
                        text: selectedSolver ? selectedSolver.name : ""
                        font.pixelSize: 22
                        font.bold: true
                        color: "#e0e0e0"
                    }

                    Text {
                        text: selectedSolver ? selectedSolver.category : ""
                        font.pixelSize: 12
                        color: "#607d8b"
                        font.letterSpacing: 1
                    }

                    Text {
                        text: selectedSolver ? selectedSolver.desc : ""
                        font.pixelSize: 14
                        color: "#b0b0b0"
                        wrapMode: Text.WordWrap
                        Layout.fillWidth: true
                    }

                    RowLayout {
                        spacing: 8
                        Text { text: "Complexity:"; color: "#607d8b"; font.pixelSize: 12 }
                        Text {
                            text: selectedSolver ? selectedSolver.complexity : ""
                            color: "#e0e0e0"
                            font.pixelSize: 12
                            font.family: "monospace"
                        }
                    }

                    // Exact solver warning
                    Rectangle {
                        visible: exactWarning
                        Layout.fillWidth: true
                        height: warningText.implicitHeight + 16
                        color: "#3a1a00"
                        radius: 6
                        border.color: "#ff6d00"
                        border.width: 1

                        Text {
                            id: warningText
                            anchors { fill: parent; margins: 8 }
                            text: "⚠  " + FileLoader.cityCount + " cities — exact solvers are practical only for n ≤ 20"
                            color: "#ff9800"
                            font.pixelSize: 12
                            wrapMode: Text.WordWrap
                        }
                    }
                }

                Item { Layout.fillHeight: true }

                // Bottom action bar
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 12

                    Button {
                        text: "← Back"
                        onClicked: backRequested()
                    }

                    Item { Layout.fillWidth: true }

                    Button {
                        text: "Configure →"
                        visible: selectedSolver !== null && selectedSolver.hasOptions
                        enabled: selectedSolver !== null && !exactWarning
                        onClicked: configureRequested()
                    }

                    Button {
                        text: "Solve ▶"
                        highlighted: true
                        visible: selectedSolver !== null && !selectedSolver.hasOptions
                        enabled: selectedSolver !== null && !exactWarning
                        onClicked: solveRequested()
                    }
                }
            }
        }
    }

    // ── Inline component: ConfigPage (issue #106) ───────────────────────────
    component ConfigPage: Page {
        signal backRequested()
        signal solveRequested()

        background: Rectangle { color: "#1a1a2e" }

        // Defaults matching Rust structs
        readonly property var saDefaults:  ({ epochs: 10000, cooling_rate: 0.0001, min_temperature: 0.001, max_temperature: 1000.0 })
        readonly property var gaDefaults:  ({ epochs: 10000, mutation_probability: 0.001, n_elite: 3 })
        readonly property var psoDefaults: ({ epochs: 10000, n_nearest: 30 })
        readonly property var csDefaults:  ({ epochs: 10000, mutation_probability: 0.001 })
        readonly property var fpaDefaults: ({ epochs: 10000, mutation_probability: 0.001 })

        property string alias: SolverEngine.selectedSolver
        property bool isSA:  alias === "sa"  || alias === "simulated_annealing"
        property bool isGA:  alias === "ga"  || alias === "genetic_algorithm"
        property bool isPSO: alias === "pso" || alias === "particle_swarm"
        property bool isCS:  alias === "cs"  || alias === "cuckoo_search"
        property bool isFPA: alias === "fpa" || alias === "flower_pollination"

        // Collect current field values as JSON for passing to Rust
        function collectOpts() {
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

        ScrollView {
            id: scrollView
            anchors.fill: parent
            anchors.bottomMargin: 64
            contentWidth: availableWidth

            ColumnLayout {
                width: scrollView.availableWidth
                spacing: 0

                // ── Header ────────────────────────────────────────────────
                Rectangle {
                    Layout.fillWidth: true
                    height: 64
                    color: "#13132b"

                    RowLayout {
                        anchors { fill: parent; leftMargin: 24; rightMargin: 24 }
                        Text {
                            text: "Configure Solver"
                            font.pixelSize: 20; font.bold: true; color: "#e0e0e0"
                        }
                        Item { Layout.fillWidth: true }
                        Rectangle {
                            width: solverChip.implicitWidth + 20; height: 28; radius: 14
                            color: "#1e3a5f"; border.color: "#4fc3f7"; border.width: 1
                            Text {
                                id: solverChip
                                anchors.centerIn: parent
                                text: SolverEngine.selectedSolver
                                color: "#4fc3f7"; font.pixelSize: 13
                            }
                        }
                    }
                }

                // ── Form body ─────────────────────────────────────────────
                ColumnLayout {
                    Layout.margins: 32
                    spacing: 28

                    // ── Common: epochs ────────────────────────────────────
                    ColumnLayout {
                        spacing: 6; Layout.fillWidth: true
                        Text { text: "Epochs"; color: "#c8c8e0"; font.pixelSize: 13; font.bold: true }
                        TextField {
                            id: epochsField
                            Layout.preferredWidth: 220
                            font.pixelSize: 14
                            text: isSA ? saDefaults.epochs.toString()
                                       : isGA ? gaDefaults.epochs.toString()
                                       : isPSO ? psoDefaults.epochs.toString()
                                       : isCS ? csDefaults.epochs.toString()
                                       : fpaDefaults.epochs.toString()
                            placeholderText: "10000"
                            color: acceptableInput ? "#111122" : "#c0302a"
                            placeholderTextColor: "#8888aa"
                            validator: IntValidator { bottom: 1; top: 10000000 }
                            background: Rectangle { color: "#dde0f5"; radius: 4; border.color: parent.activeFocus ? "#4fc3f7" : "#9090c0"; border.width: 1 }
                        }
                        Text { text: "Number of iterations the solver will run"; color: "#7890a8"; font.pixelSize: 11 }
                    }

                    // ── SA fields ─────────────────────────────────────────
                    ColumnLayout {
                        visible: isSA
                        spacing: 20; Layout.fillWidth: true

                        ColumnLayout {
                            spacing: 6; Layout.fillWidth: true
                            Text { text: "Cooling rate"; color: "#c8c8e0"; font.pixelSize: 13; font.bold: true }
                            TextField {
                                id: crField
                                Layout.preferredWidth: 220
                                font.pixelSize: 14
                                text: saDefaults.cooling_rate.toString()
                                placeholderText: "0.0001"
                                color: acceptableInput ? "#111122" : "#c0302a"
                                placeholderTextColor: "#8888aa"
                                validator: DoubleValidator { bottom: 0.000001; top: 0.999999; notation: DoubleValidator.StandardNotation }
                                background: Rectangle { color: "#dde0f5"; radius: 4; border.color: parent.activeFocus ? "#4fc3f7" : "#9090c0"; border.width: 1 }
                            }
                            Text { text: "Must be > 0 and < 1"; color: "#7890a8"; font.pixelSize: 11 }
                        }

                        RowLayout {
                            spacing: 32; Layout.fillWidth: true
                            ColumnLayout {
                                spacing: 6
                                Text { text: "Min temperature"; color: "#c8c8e0"; font.pixelSize: 13; font.bold: true }
                                TextField {
                                    id: minTField
                                    width: 180; font.pixelSize: 14
                                    text: saDefaults.min_temperature.toString()
                                    placeholderText: "0.001"
                                    color: "#111122"; placeholderTextColor: "#8888aa"
                                    validator: DoubleValidator { bottom: 0; notation: DoubleValidator.StandardNotation }
                                    background: Rectangle { color: "#dde0f5"; radius: 4; border.color: parent.activeFocus ? "#4fc3f7" : "#9090c0"; border.width: 1 }
                                }
                            }
                            ColumnLayout {
                                spacing: 6
                                Text { text: "Max temperature"; color: "#c8c8e0"; font.pixelSize: 13; font.bold: true }
                                TextField {
                                    id: maxTField
                                    width: 180; font.pixelSize: 14
                                    text: saDefaults.max_temperature.toString()
                                    placeholderText: "1000.0"
                                    color: "#111122"; placeholderTextColor: "#8888aa"
                                    validator: DoubleValidator { bottom: 0.000001; notation: DoubleValidator.StandardNotation }
                                    background: Rectangle { color: "#dde0f5"; radius: 4; border.color: parent.activeFocus ? "#4fc3f7" : "#9090c0"; border.width: 1 }
                                }
                            }
                        }
                        Text {
                            visible: { var mn = parseFloat(minTField.text); var mx = parseFloat(maxTField.text); return !isNaN(mn) && !isNaN(mx) && mn >= mx }
                            text: "⚠  Min temperature must be less than max temperature"
                            color: "#ef5350"; font.pixelSize: 12
                        }
                    }

                    // ── GA fields ─────────────────────────────────────────
                    ColumnLayout {
                        visible: isGA
                        spacing: 20; Layout.fillWidth: true

                        ColumnLayout {
                            spacing: 6; Layout.fillWidth: true
                            Text { text: "Mutation probability"; color: "#c8c8e0"; font.pixelSize: 13; font.bold: true }
                            TextField {
                                id: mpField
                                Layout.preferredWidth: 220; font.pixelSize: 14
                                text: gaDefaults.mutation_probability.toString()
                                placeholderText: "0.001"
                                color: acceptableInput ? "#111122" : "#c0302a"; placeholderTextColor: "#8888aa"
                                validator: DoubleValidator { bottom: 0; top: 1; notation: DoubleValidator.StandardNotation }
                                background: Rectangle { color: "#dde0f5"; radius: 4; border.color: parent.activeFocus ? "#4fc3f7" : "#9090c0"; border.width: 1 }
                            }
                            Text { text: "Range [0, 1]"; color: "#7890a8"; font.pixelSize: 11 }
                        }

                        ColumnLayout {
                            spacing: 6
                            Text { text: "Elite count"; color: "#c8c8e0"; font.pixelSize: 13; font.bold: true }
                            TextField {
                                id: eliteField
                                width: 140; font.pixelSize: 14
                                text: gaDefaults.n_elite.toString()
                                placeholderText: "3"
                                color: acceptableInput ? "#111122" : "#c0302a"; placeholderTextColor: "#8888aa"
                                validator: IntValidator { bottom: 1; top: 1000 }
                                background: Rectangle { color: "#dde0f5"; radius: 4; border.color: parent.activeFocus ? "#4fc3f7" : "#9090c0"; border.width: 1 }
                            }
                            Text { text: "Elite solutions preserved each generation"; color: "#7890a8"; font.pixelSize: 11 }
                        }
                    }

                    // ── CS / FPA shared mutation field ────────────────────
                    ColumnLayout {
                        visible: isCS || isFPA
                        spacing: 6; Layout.fillWidth: true
                        Text { text: "Mutation probability"; color: "#c8c8e0"; font.pixelSize: 13; font.bold: true }
                        TextField {
                            id: mpField2
                            Layout.preferredWidth: 220; font.pixelSize: 14
                            text: isCS ? csDefaults.mutation_probability.toString()
                                       : fpaDefaults.mutation_probability.toString()
                            placeholderText: "0.001"
                            color: acceptableInput ? "#111122" : "#c0302a"; placeholderTextColor: "#8888aa"
                            validator: DoubleValidator { bottom: 0; top: 1; notation: DoubleValidator.StandardNotation }
                            background: Rectangle { color: "#dde0f5"; radius: 4; border.color: parent.activeFocus ? "#4fc3f7" : "#9090c0"; border.width: 1 }
                        }
                        Text { text: "Range [0, 1]"; color: "#7890a8"; font.pixelSize: 11 }
                    }

                    // ── PSO fields ────────────────────────────────────────
                    ColumnLayout {
                        visible: isPSO
                        spacing: 6
                        Text { text: "Swarm size (min)"; color: "#c8c8e0"; font.pixelSize: 13; font.bold: true }
                        TextField {
                            id: swarmField
                            width: 140; font.pixelSize: 14
                            text: psoDefaults.n_nearest.toString()
                            placeholderText: "30"
                            color: acceptableInput ? "#111122" : "#c0302a"; placeholderTextColor: "#8888aa"
                            validator: IntValidator { bottom: 1; top: 10000 }
                            background: Rectangle { color: "#dde0f5"; radius: 4; border.color: parent.activeFocus ? "#4fc3f7" : "#9090c0"; border.width: 1 }
                        }
                        Text { text: "Minimum particle count (floor is 30)"; color: "#7890a8"; font.pixelSize: 11 }
                    }
                }
            }
        }

        // ── Bottom bar ────────────────────────────────────────────────────
        Rectangle {
            anchors.bottom: parent.bottom
            anchors.left: parent.left
            anchors.right: parent.right
            height: 56
            color: "#0d0d1e"
            z: 10

            RowLayout {
                anchors { fill: parent; leftMargin: 20; rightMargin: 20 }
                spacing: 12

                Button { text: "← Back"; onClicked: backRequested() }

                Item { Layout.fillWidth: true }

                Text {
                    visible: hasError()
                    text: "Fix validation errors above"
                    color: "#ef5350"; font.pixelSize: 12
                }

                Button {
                    text: "Solve ▶"
                    highlighted: true
                    enabled: !hasError()
                    onClicked: {
                        SolverEngine.startSolveWithOpts(FileLoader.filePath, collectOpts())
                        solveRequested()
                    }
                }
            }
        }
    }

    // ── Inline component: PipelinePage (issue #107) ─────────────────────────
    component PipelinePage: Page {
        signal backRequested()

        background: Rectangle { color: "#1a1a2e" }

        // Each stage: { solver: "nn", name: "Nearest Neighbor", category: "Constructive" }
        property var stages: []

        function stagesJson() {
            return JSON.stringify(stages.map(function(s) { return { solver: s.solver } }))
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 0
            spacing: 0

            // ── Header ────────────────────────────────────────────────────
            Rectangle {
                Layout.fillWidth: true
                height: 60
                color: "#13132b"
                RowLayout {
                    anchors { fill: parent; leftMargin: 24; rightMargin: 24 }
                    Text { text: "Pipeline Builder"; font.pixelSize: 20; font.bold: true; color: "#e0e0e0" }
                    Item { Layout.fillWidth: true }
                    Text {
                        text: stages.length + " stage" + (stages.length === 1 ? "" : "s")
                        color: "#607d8b"; font.pixelSize: 13
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
                            font.pixelSize: 18; color: "#555"
                        }
                        Text {
                            Layout.alignment: Qt.AlignHCenter
                            text: "Press + to add a solver stage"
                            font.pixelSize: 13; color: "#444"
                        }
                    }

                    // Stage cards
                    Repeater {
                        model: stages
                        delegate: Rectangle {
                            required property var modelData
                            required property int index
                            width: parent ? parent.width - 48 : 400
                            height: 64
                            radius: 6
                            color: "#16213e"
                            border.color: "#2a3a6a"
                            border.width: 1

                            RowLayout {
                                anchors { fill: parent; leftMargin: 16; rightMargin: 8 }
                                spacing: 12

                                // Stage number badge
                                Rectangle {
                                    width: 28; height: 28; radius: 14
                                    color: "#1e3a5f"
                                    Text {
                                        anchors.centerIn: parent
                                        text: (index + 1).toString()
                                        color: "#4fc3f7"; font.pixelSize: 13; font.bold: true
                                    }
                                }

                                // Arrow between stages
                                Text {
                                    visible: index > 0
                                    text: "→"
                                    color: "#607d8b"; font.pixelSize: 14
                                    // Note: shown as part of this card for simplicity
                                }

                                ColumnLayout {
                                    spacing: 2
                                    Layout.fillWidth: true
                                    Text {
                                        text: modelData.name
                                        color: "#e0e0e0"; font.pixelSize: 14; font.bold: true
                                    }
                                    Text {
                                        text: modelData.category
                                        color: "#607d8b"; font.pixelSize: 11
                                    }
                                }

                                // Move up
                                Button {
                                    text: "↑"
                                    enabled: index > 0
                                    flat: true
                                    onClicked: {
                                        var arr = stages.slice()
                                        var tmp = arr[index - 1]
                                        arr[index - 1] = arr[index]
                                        arr[index] = tmp
                                        stages = arr
                                    }
                                }

                                // Move down
                                Button {
                                    text: "↓"
                                    enabled: index < stages.length - 1
                                    flat: true
                                    onClicked: {
                                        var arr = stages.slice()
                                        var tmp = arr[index + 1]
                                        arr[index + 1] = arr[index]
                                        arr[index] = tmp
                                        stages = arr
                                    }
                                }

                                // Remove
                                Button {
                                    text: "✕"
                                    flat: true
                                    onClicked: {
                                        var arr = stages.slice()
                                        arr.splice(index, 1)
                                        stages = arr
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
                color: "#0d0d1e"

                RowLayout {
                    anchors { fill: parent; leftMargin: 20; rightMargin: 20 }
                    spacing: 12

                    Button { text: "← Back"; onClicked: backRequested() }

                    Item { Layout.fillWidth: true }

                    Text {
                        visible: stages.length < 2
                        text: "Add at least 2 stages to run"
                        color: "#607d8b"; font.pixelSize: 12
                    }

                    Button {
                        text: "Run Pipeline ▶"
                        highlighted: true
                        enabled: stages.length >= 1
                        onClicked: {
                            SolverEngine.runPipelineStages(FileLoader.filePath, stagesJson())
                            stackView.push(vizComp)
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

            background: Rectangle { color: "#13132b"; radius: 8; border.color: "#2a3a6a"; border.width: 1 }

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 16
                spacing: 8

                Text { text: "Add a stage"; font.pixelSize: 16; font.bold: true; color: "#e0e0e0" }

                ListView {
                    id: solverListView2
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    model: solverList
                    clip: true
                    spacing: 2

                    delegate: ItemDelegate {
                        width: solverListView2.width
                        height: 40
                        required property var modelData

                        contentItem: ColumnLayout {
                            spacing: 1
                            Text { text: modelData.name; color: "#e0e0e0"; font.pixelSize: 13 }
                            Text { text: modelData.category; color: "#607d8b"; font.pixelSize: 10 }
                        }

                        background: Rectangle {
                            color: parent.hovered ? "#1e3a5f" : "transparent"; radius: 4
                        }

                        onClicked: {
                            var arr = stages.slice()
                            arr.push({ solver: modelData.alias, name: modelData.name, category: modelData.category })
                            stages = arr
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

        background: Rectangle { color: "#0f0f23" }

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
                ctx.fillStyle = "#0f0f23"
                ctx.fillRect(0, 0, width, height)

                var pos = cityPos
                var ids = Object.keys(pos)
                if (!ids.length) return

                // Tour edges
                var tourRaw = SolverEngine.tourJson
                if (tourRaw && tourRaw !== "[]") {
                    var tour = JSON.parse(tourRaw)
                    if (tour.length > 1) {
                        ctx.strokeStyle = "#00e676"
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

                // City dots
                ctx.fillStyle = "#4fc3f7"
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
            }
        }

        // ── KPI overlay (top-right) ────────────────────────────────────────
        Rectangle {
            anchors.top: parent.top
            anchors.right: parent.right
            anchors.margins: 12
            width: kpiCol.implicitWidth + 24
            height: kpiCol.implicitHeight + 20
            color: "#cc0a0a1e"
            radius: 8
            border.color: "#2a2a5a"
            border.width: 1
            z: 10

            ColumnLayout {
                id: kpiCol
                anchors.centerIn: parent
                spacing: 6

                RowLayout {
                    spacing: 12
                    Text { text: "Solver"; color: "#607d8b"; font.pixelSize: 11 }
                    Text { text: SolverEngine.selectedSolver; color: "#4fc3f7"; font.pixelSize: 13; font.bold: true }
                }
                RowLayout {
                    spacing: 12
                    Text { text: "Best cost"; color: "#607d8b"; font.pixelSize: 11 }
                    Text {
                        text: SolverEngine.bestCost > 0 ? SolverEngine.bestCost.toFixed(2) : "—"
                        color: "#00e676"; font.pixelSize: 15; font.bold: true
                    }
                }
                RowLayout {
                    spacing: 12
                    Text { text: "Iteration"; color: "#607d8b"; font.pixelSize: 11 }
                    Text { text: SolverEngine.iteration.toString(); color: "#e0e0e0"; font.pixelSize: 13 }
                }
                RowLayout {
                    spacing: 12
                    Text { text: "Elapsed"; color: "#607d8b"; font.pixelSize: 11 }
                    Text {
                        text: {
                            var ms = SolverEngine.elapsedMs
                            if (ms < 1000) return ms + " ms"
                            return (ms / 1000).toFixed(1) + " s"
                        }
                        color: "#e0e0e0"; font.pixelSize: 13
                    }
                }

                // Running indicator
                RowLayout {
                    visible: SolverEngine.running
                    spacing: 6
                    Rectangle {
                        width: 8; height: 8; radius: 4; color: "#00e676"
                        SequentialAnimation on opacity {
                            loops: Animation.Infinite
                            NumberAnimation { to: 0.2; duration: 600 }
                            NumberAnimation { to: 1.0; duration: 600 }
                        }
                    }
                    Text { text: "Solving…"; color: "#00e676"; font.pixelSize: 11 }
                }
            }
        }

        // ── Bottom bar ─────────────────────────────────────────────────────
        Rectangle {
            anchors.bottom: parent.bottom
            anchors.left: parent.left
            anchors.right: parent.right
            height: 56
            color: "#0d0d1e"
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
                    color: "#00e676"
                    font.pixelSize: 15
                    font.bold: true
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
            onNextRequested:     stackView.push(solverComp)
            onPipelineRequested: stackView.push(pipelineComp)
        }
    }

    Component {
        id: pipelineComp
        PipelinePage { onBackRequested: stackView.pop() }
    }

    Component {
        id: solverComp
        SolverPage {
            onBackRequested:    stackView.pop()
            onConfigureRequested: stackView.push(configComp)
            onSolveRequested: {
                SolverEngine.startSolve(FileLoader.filePath)
                stackView.push(vizComp)
            }
        }
    }

    Component {
        id: configComp
        ConfigPage {
            onBackRequested:  stackView.pop()
            onSolveRequested: stackView.push(vizComp)
        }
    }

    Component {
        id: vizComp
        VisualizationPage { onBackRequested: stackView.pop() }
    }
}
