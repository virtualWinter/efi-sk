import QtQuick
import QtQuick.Controls.Basic as QQC2
import QtQuick.Layouts

// installer is injected by the Rust QML engine at runtime.
// qmllint disable unqualified
QQC2.ApplicationWindow {
    id: window
    visible: true
    width: 800
    height: 570
    minimumWidth: 720
    minimumHeight: 540
    title: "EFI Swiss Knife"
    color: "#0a0a0a"
    font.family: Qt.fontFamilies().includes("Geist") ? "Geist" : Qt.application.font.family
    font.pixelSize: 14
    palette.window: color
    palette.base: "#171717"
    palette.button: "#171717"
    palette.mid: "#2e2e2e"
    palette.highlight: "#262626"
    palette.highlightedText: "#fafafa"
    palette.text: "#fafafa"
    palette.windowText: "#fafafa"
    palette.buttonText: "#fafafa"
    palette.placeholderText: "#a1a1a1"

    component Section: QQC2.GroupBox {
        id: section
        Layout.fillWidth: true
        padding: 16
        topPadding: 44
        label: QQC2.Label {
            x: 16
            y: 14
            text: section.title
            color: "#fafafa"
            font.weight: Font.DemiBold
        }
        background: Rectangle {
            radius: 12
            color: "#171717"
            border.color: "#2e2e2e"
        }
    }

    component Muted: QQC2.Label {
        color: "#a1a1a1"
        wrapMode: Text.Wrap
        Layout.fillWidth: true
    }

    component Button: QQC2.Button {
        id: button
        property bool primary: false
        property bool destructive: false
        implicitHeight: 36
        leftPadding: 14
        rightPadding: 14
        font.weight: Font.DemiBold
        opacity: enabled ? 1 : 0.45
        contentItem: QQC2.Label {
            text: button.text
            color: button.primary || button.destructive ? "#0a0a0a" : "#fafafa"
            font: button.font
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
        }
        background: Rectangle {
            radius: 7
            color: button.destructive ? (button.hovered ? "#ff8588" : "#ff6467") : button.primary ? (button.hovered ? "#dedede" : "#fafafa") : button.down || button.hovered ? "#262626" : "#171717"
            border.color: button.visualFocus ? "#a1a1a1" : button.primary || button.destructive ? color : "#2e2e2e"
            border.width: button.visualFocus ? 2 : 1
        }
    }

    property string selectedIsoIndexes: ""

    function toggleIso(index, checked) {
        var values = selectedIsoIndexes ? selectedIsoIndexes.split(",") : [];
        var position = values.indexOf(String(index));
        if (checked && position < 0)
            values.push(String(index));
        else if (!checked && position >= 0)
            values.splice(position, 1);
        selectedIsoIndexes = values.join(",");
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 18
        spacing: 10

        QQC2.Label {
            text: "Create a bootable USB"
            font.pixelSize: 30
            font.weight: Font.DemiBold
            Layout.fillWidth: true
        }
        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 10

            Section {
                title: "Boot images"
                Layout.fillHeight: true
                Layout.preferredWidth: 440
                Layout.minimumWidth: 330
                contentItem: ColumnLayout {
                    spacing: 8
                    Muted {
                        text: "Select images and download them before installing."
                    }
                    ListView {
                        id: imageList
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        Layout.minimumHeight: 0
                        clip: true
                        boundsBehavior: Flickable.StopAtBounds
                        QQC2.ScrollBar.vertical: QQC2.ScrollBar {}
                        model: installer.iso_names.split("\n").map((name, index) => ({
                                    name: name,
                                    sourceIndex: index
                                })).sort((a, b) => a.name.localeCompare(b.name))
                        spacing: 1
                        delegate: Rectangle {
                            required property var modelData
                            readonly property int sourceIndex: modelData.sourceIndex
                            width: imageList.width - 14
                            height: 44
                            radius: 7
                            color: isoCheck.checked || rowHover.hovered ? "#262626" : "transparent"
                            border.color: isoCheck.checked ? "#737373" : "transparent"
                            HoverHandler {
                                id: rowHover
                            }
                            RowLayout {
                                anchors.fill: parent
                                anchors.leftMargin: 8
                                anchors.rightMargin: 6
                                spacing: 8
                                Image {
                                    source: "qrc:/iso-icons/" + installer.iso_icons.split("\n")[sourceIndex] + ".png"
                                    Layout.preferredWidth: 20
                                    Layout.preferredHeight: 20
                                    fillMode: Image.PreserveAspectFit
                                    smooth: true
                                }
                                QQC2.CheckBox {
                                    id: isoCheck
                                    checked: window.selectedIsoIndexes.split(",").indexOf(String(sourceIndex)) >= 0
                                    onToggled: window.toggleIso(sourceIndex, checked)
                                    Accessible.name: "Include " + modelData.name
                                }
                                QQC2.Label {
                                    text: modelData.name
                                    Layout.fillWidth: true
                                    elide: Text.ElideMiddle
                                }
                                Button {
                                    text: "Download"
                                    onClicked: installer.download_iso(sourceIndex)
                                }
                            }
                        }
                    }
                }
            }

            Section {
                title: "Optional EFI tools"
                Layout.fillHeight: true
                Layout.preferredWidth: 300
                Layout.minimumWidth: 300
                contentItem: ColumnLayout {
                    spacing: 6
                    RowLayout {
                        visible: installer.release_mode
                        Layout.fillWidth: true
                        QQC2.Label {
                            text: "EFI release version"
                        }
                        QQC2.TextField {
                            id: efiVersion
                            Layout.fillWidth: true
                            Layout.minimumWidth: 0
                            implicitHeight: 34
                            placeholderText: "latest or v0.1.1"
                            Accessible.name: "EFI release version"
                        }
                    }
                    RowLayout {
                        Layout.fillWidth: true
                        Muted {
                            text: "Boot picker always included."
                        }
                        Button {
                            text: "Deselect all tools"
                            onClicked: {
                                for (let i = 0; i < toolList.count; ++i)
                                    (toolList.itemAt(i) as QQC2.CheckBox).checked = false;
                            }
                        }
                    }
                    GridLayout {
                        Layout.fillWidth: true
                        columns: 2
                        rowSpacing: 0
                        Repeater {
                            id: toolList
                            model: installer.tool_names.split("\n")
                            delegate: QQC2.CheckBox {
                                required property string modelData
                                Layout.fillWidth: true
                                Layout.preferredWidth: 1
                                Layout.minimumWidth: 0
                                implicitHeight: 28
                                topPadding: 0
                                bottomPadding: 0
                                text: modelData.startsWith("UEFI SCT") ? "UEFI SCT" : modelData
                                checked: true
                            }
                        }
                    }
                    Muted {
                        text: "UEFI SCT also installs EFI Shell."
                    }
                    Item {
                        Layout.fillHeight: true
                    }
                }
            }
        }

        Section {
            title: "USB drive"
            contentItem: RowLayout {
                QQC2.ComboBox {
                    id: diskBox
                    Layout.fillWidth: true
                    Layout.minimumWidth: 0
                    model: installer.devices ? installer.devices.split("\n") : []
                    displayText: count === 0 ? "No USB drive detected" : currentText
                    Accessible.name: "USB drive"
                }
                Button {
                    text: "Refresh"
                    onClicked: installer.refresh()
                }
                Button {
                    text: "Create USB"
                    primary: true
                    enabled: diskBox.currentIndex >= 0 && window.selectedIsoIndexes.length > 0
                    onClicked: eraseDialog.open()
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            QQC2.Label {
                id: statusLabel
                Layout.fillWidth: true
                Layout.minimumWidth: 0
                text: (window.selectedIsoIndexes ? window.selectedIsoIndexes.split(",").length + " image(s) selected" : "No images selected") + "  ·  " + installer.status
                color: "#a1a1a1"
                elide: Text.ElideRight
            }
            Button {
                text: "Details"
                visible: statusLabel.truncated
                onClicked: statusDialog.open()
            }
        }
    }
    QQC2.Dialog {
        id: statusDialog
        anchors.centerIn: parent
        modal: true
        width: Math.min(window.width - 48, 460)
        height: Math.min(window.height - 48, 330)
        title: "Installer status"
        standardButtons: QQC2.Dialog.Close
        contentItem: QQC2.ScrollView {
            clip: true
            contentWidth: availableWidth
            QQC2.TextArea {
                text: installer.status
                readOnly: true
                wrapMode: TextEdit.Wrap
            }
        }
    }
    QQC2.Dialog {
        id: eraseDialog
        anchors.centerIn: parent
        modal: true
        width: Math.min(window.width - 48, 460)
        padding: 20
        title: "Erase USB drive?"
        background: Rectangle {
            radius: 12
            color: "#171717"
            border.color: "#2e2e2e"
        }
        contentItem: ColumnLayout {
            spacing: 16
            Muted {
                text: diskBox.currentText + "\n\nSelected images and tools will be installed. Existing data will be permanently erased."
            }
            RowLayout {
                Layout.alignment: Qt.AlignRight
                Button {
                    text: "Cancel"
                    focus: true
                    onClicked: eraseDialog.reject()
                }
                Button {
                    text: "Erase and install"
                    destructive: true
                    onClicked: eraseDialog.accept()
                }
            }
        }
        onAccepted: {
            let excluded = [];
            for (let i = 0; i < toolList.count; ++i) {
                if (!(toolList.itemAt(i) as QQC2.CheckBox).checked)
                    excluded.push(i);
            }
            installer.create_usb(window.selectedIsoIndexes, diskBox.currentIndex, excluded.join(","), efiVersion.text);
        }
    }
}
