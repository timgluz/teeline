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
                    text: "Advanced (Pipeline Builder)"
                    enabled: false
                    opacity: 0.4
                }
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
        { name: "Tabu Search",          alias: "tabu",           category: "Metaheuristic",
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

    // ── Inline component: ConfigPage (placeholder — issue #106) ─────────────
    component ConfigPage: Page {
        signal backRequested()
        signal solveRequested()
        background: Rectangle { color: "#1a1a2e" }
        ColumnLayout {
            anchors.centerIn: parent
            spacing: 16
            Text { text: "Solver Config"; font.pixelSize: 24; color: "#e0e0e0"; Layout.alignment: Qt.AlignHCenter }
            Text { text: "Coming in issue #106"; font.pixelSize: 14; color: "#9e9e9e"; Layout.alignment: Qt.AlignHCenter }
            Text { text: "Solver: " + SolverEngine.selectedSolver; font.pixelSize: 13; color: "#4fc3f7"; Layout.alignment: Qt.AlignHCenter }
            RowLayout {
                Layout.alignment: Qt.AlignHCenter
                spacing: 12
                Button { text: "← Back"; onClicked: backRequested() }
                Button { text: "Solve ▶"; highlighted: true; onClicked: solveRequested() }
            }
        }
    }

    // ── Inline component: VisualizationPage (placeholder — issue #105) ──────
    component VisualizationPage: Page {
        signal backRequested()
        background: Rectangle { color: "#0f0f23" }
        ColumnLayout {
            anchors.centerIn: parent
            spacing: 16
            Text { text: "Visualization"; font.pixelSize: 24; color: "#e0e0e0"; Layout.alignment: Qt.AlignHCenter }
            Text { text: "Coming in issue #105"; font.pixelSize: 14; color: "#9e9e9e"; Layout.alignment: Qt.AlignHCenter }
            Text { text: "Solver: " + SolverEngine.selectedSolver; font.pixelSize: 13; color: "#00e676"; Layout.alignment: Qt.AlignHCenter }
            Button { text: "← Back"; Layout.alignment: Qt.AlignHCenter; onClicked: backRequested() }
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
        WelcomePage { onNextRequested: stackView.push(solverComp) }
    }

    Component {
        id: solverComp
        SolverPage {
            onBackRequested:    stackView.pop()
            onConfigureRequested: stackView.push(configComp)
            onSolveRequested:   stackView.push(vizComp)
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
