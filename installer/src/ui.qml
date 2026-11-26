import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts

// installer is injected by the Rust QML engine at runtime.
// qmllint disable unqualified
QQC2.ApplicationWindow {
    id: window
    Timer {
        interval: 200
        running: installer.busy
        repeat: true
        onTriggered: installer.poll_updates()
    }
    visible: true
    width: 800
    height: 570
    minimumWidth: 730
    minimumHeight: 550
    title: "EFI Swiss Knife"
    component Section: QQC2.Control {
        id: section
        property string title
        Layout.fillWidth: true
        topPadding: heading.implicitHeight + 8

        QQC2.Label {
            id: heading
            text: section.title
            font.bold: true
            Accessible.role: Accessible.Heading
        }
    }

    component Muted: QQC2.Label {
        color: palette.placeholderText
        wrapMode: Text.Wrap
        Layout.fillWidth: true
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

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 20

            Section {
                title: "Boot images (optional)"
                Layout.fillHeight: true
                Layout.preferredWidth: 440
                Layout.minimumWidth: 330
                contentItem: ColumnLayout {
                    spacing: 8
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
                            height: 36
                            color: isoCheck.checked || rowHover.hovered ? window.palette.alternateBase : "transparent"
                            HoverHandler {
                                id: rowHover
                            }
                            RowLayout {
                                anchors.fill: parent
                                anchors.leftMargin: 8
                                anchors.rightMargin: 6
                                spacing: 8
                                QQC2.CheckBox {
                                    id: isoCheck
                                    visible: !modelData.name.endsWith(": Download only")
                                    checked: window.selectedIsoIndexes.split(",").indexOf(String(sourceIndex)) >= 0
                                    onToggled: window.toggleIso(sourceIndex, checked)
                                    Accessible.name: "Include " + modelData.name
                                }
                                Image {
                                    source: "qrc:/iso-icons/" + installer.iso_icons.split("\n")[sourceIndex] + ".png"
                                    Layout.preferredWidth: 18
                                    Layout.preferredHeight: 18
                                    fillMode: Image.PreserveAspectFit
                                    smooth: true
                                }
                                QQC2.Label {
                                    text: modelData.name
                                    Layout.fillWidth: true
                                    elide: Text.ElideMiddle
                                    QQC2.ToolTip.visible: isoNameHover.hovered && truncated
                                    QQC2.ToolTip.text: text
                                    HoverHandler {
                                        id: isoNameHover
                                    }
                                }
                                QQC2.Button {
                                    text: "Download"
                                    icon.name: "download"
                                    visible: modelData.name.endsWith(": Download only")
                                    enabled: !installer.busy
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
                Layout.preferredWidth: 350
                Layout.minimumWidth: 320
                contentItem: ColumnLayout {
                    spacing: 12
                    RowLayout {
                        visible: installer.release_mode
                        Layout.fillWidth: true
                        QQC2.Label {
                            text: "EFI version"
                        }
                        QQC2.TextField {
                            id: efiVersion
                            Layout.fillWidth: true
                            Layout.minimumWidth: 0
                            implicitHeight: 34
                            placeholderText: "latest or v0.1.1"
                            Accessible.name: "EFI version"
                        }
                    }
                    RowLayout {
                        Layout.fillWidth: true
                        Item { Layout.fillWidth: true }
                        QQC2.Button {
                            text: "Clear all"
                            Accessible.name: "Deselect all EFI tools"
                            onClicked: {
                                for (let i = 0; i < toolList.count; ++i)
                                    (toolList.itemAt(i) as QQC2.CheckBox).checked = false;
                            }
                        }
                    }
                    GridLayout {
                        Layout.fillWidth: true
                        columns: 2
                        rowSpacing: 4
                        columnSpacing: 8
                        Repeater {
                            id: toolList
                            model: installer.tool_names.split("\n")
                            delegate: QQC2.CheckBox {
                                id: toolCheck
                                required property string modelData
                                Layout.fillWidth: true
                                Layout.minimumWidth: 0
                                Layout.preferredWidth: 1
                                implicitHeight: 30
                                padding: 4
                                text: modelData.startsWith("UEFI SCT") ? "UEFI SCT" : modelData
                                checked: true
                                QQC2.ToolTip.visible: (hovered || visualFocus) && text === "UEFI SCT"
                                QQC2.ToolTip.text: "Also installs EFI Shell."
                            }
                        }
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
                    implicitHeight: 36
                    model: installer.devices ? installer.devices.split("\n") : []
                    currentIndex: -1
                    onModelChanged: currentIndex = -1
                    enabled: count > 0
                    displayText: count === 0 ? "No USB drive detected" : currentIndex < 0 ? "Choose a USB drive…" : currentText
                    Accessible.name: "USB drive"
                }
                QQC2.Button {
                    text: "Refresh"
                    icon.name: "view-refresh"
                    onClicked: installer.refresh()
                }
                QQC2.Button {
                    text: "Create USB"
                    icon.name: "drive-removable-media"
                    highlighted: true
                    enabled: !installer.busy && diskBox.currentIndex >= 0
                    onClicked: eraseDialog.open()
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            visible: installer.status.length > 0 && !installer.installing
            QQC2.BusyIndicator {
                running: installer.busy
                visible: running
                Layout.preferredWidth: 24
                Layout.preferredHeight: 24
            }
            QQC2.Label {
                id: statusLabel
                Layout.fillWidth: true
                Layout.minimumWidth: 0
                text: installer.status
                color: palette.placeholderText
                elide: Text.ElideRight
            }
            QQC2.Button {
                text: "Details"
                visible: statusLabel.truncated
                onClicked: statusDialog.open()
            }
        }
    }
    QQC2.Dialog {
        id: installProgress
        anchors.centerIn: parent
        width: Math.min(window.width - 48, 440)
        padding: 24
        modal: true
        dim: true
        focus: true
        visible: installer.installing
        closePolicy: QQC2.Popup.NoAutoClose
        title: "Installing to USB"
        contentItem: ColumnLayout {
            spacing: 16
            QQC2.Label {
                Layout.fillWidth: true
                text: installer.status
                wrapMode: Text.WrapAnywhere
            }
            QQC2.ProgressBar {
                Layout.fillWidth: true
                indeterminate: installProgress.visible
                Accessible.name: "USB installation progress"
            }
            Muted { text: "Keep the drive connected until installation finishes." }
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
        contentItem: ColumnLayout {
            spacing: 16
            Muted {
                text: diskBox.currentText + "\n\nThis permanently erases all data on this drive, then installs the boot picker and your selected images and tools."
            }
        }
        footer: QQC2.DialogButtonBox {
            QQC2.Button {
                text: "Cancel"
                icon.name: "dialog-cancel"
                focus: true
                QQC2.DialogButtonBox.buttonRole: QQC2.DialogButtonBox.RejectRole
            }
            QQC2.Button {
                text: "Erase and install"
                icon.name: "edit-delete"
                QQC2.DialogButtonBox.buttonRole: QQC2.DialogButtonBox.AcceptRole
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
