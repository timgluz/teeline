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

    // ── Inline component: SolverPage (placeholder — issue #104) ────────────
    component SolverPage: Page {
        signal backRequested()
        background: Rectangle { color: "#1a1a2e" }

        ColumnLayout {
            anchors.centerIn: parent
            spacing: 16
            Text {
                text: "Solver Picker"
                font.pixelSize: 24
                color: "#e0e0e0"
                Layout.alignment: Qt.AlignHCenter
            }
            Text {
                text: "Coming in issue #104"
                font.pixelSize: 14
                color: "#9e9e9e"
                Layout.alignment: Qt.AlignHCenter
            }
            Button {
                text: "← Back"
                Layout.alignment: Qt.AlignHCenter
                onClicked: backRequested()
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
        id: solverComp
        SolverPage {
            onBackRequested: stackView.pop()
        }
    }
}
