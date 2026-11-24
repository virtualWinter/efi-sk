import QtQuick
import QtQuick.Controls.Basic as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

// installer is injected by the Rust QML engine at runtime.
// qmllint disable unqualified
QQC2.ApplicationWindow {
    id: window
    visible: true
    width: 840
    height: 760
    minimumWidth: 620
    minimumHeight: 560
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

    QQC2.ScrollView {
        anchors.fill: parent
        leftPadding: 24
        rightPadding: 24
        topPadding: 22
        bottomPadding: 22
        contentWidth: availableWidth
        clip: true

        ColumnLayout {
            width: Math.min(parent.width, 700)
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: 14

            Muted {
                text: "EFI SWISS KNIFE  /  INSTALLER"
                font.pixelSize: 11
                font.letterSpacing: 1.5
            }
            QQC2.Label {
                text: "Create a bootable USB"
                font.pixelSize: 30
                font.weight: Font.DemiBold
                Layout.fillWidth: true
            }
            Muted {
                text: "Choose the images you want to carry with you, then install them on a USB drive."
            }

            Section {
                title: "Boot images"
                contentItem: ColumnLayout {
                    spacing: 8
                    Muted {
                        text: "Select one or more images. Downloaded images are resumed and verified."
                    }
                    ListView {
                        id: imageList
                        Layout.fillWidth: true
                        Layout.preferredHeight: Math.min(contentHeight, 220)
                        implicitHeight: contentHeight
                        clip: true
                        model: installer.iso_names.split("\n")
                        spacing: 1
                        delegate: Rectangle {
                            required property int index
                            required property string modelData
                            width: imageList.width
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
                                Kirigami.Icon {
                                    source: installer.iso_icons.split("\n")[index]
                                    implicitWidth: Kirigami.Units.iconSizes.medium
                                    implicitHeight: Kirigami.Units.iconSizes.medium
                                    fallback: "application-x-iso9660-image"
                                }
                                QQC2.CheckBox {
                                    id: isoCheck
                                    checked: window.selectedIsoIndexes.split(",").indexOf(String(index)) >= 0
                                    onToggled: window.toggleIso(index, checked)
                                    Accessible.name: "Include " + modelData
                                }
                                QQC2.Label {
                                    text: modelData
                                    Layout.fillWidth: true
                                    elide: Text.ElideMiddle
                                }
                                Button {
                                    text: "Download"
                                    onClicked: installer.download_iso(index)
                                }
                            }
                        }
                    }
                }
            }

            Section {
                title: "Optional EFI tools"
                contentItem: ColumnLayout {
                    QQC2.Label {
                        text: "EFI release version"
                        visible: installer.release_mode
                    }
                    QQC2.TextField {
                        id: efiVersion
                        visible: installer.release_mode
                        Layout.fillWidth: true
                        placeholderText: "latest (or a tag, e.g. v0.1.1)"
                        Accessible.name: "EFI release version"
                    }
                    QQC2.Label {
                        text: "Choose which tools to install. The boot picker is always included."
                        Layout.fillWidth: true
                        wrapMode: Text.Wrap
                    }
                    Button {
                        text: "Deselect all tools"
                        onClicked: {
                            for (let i = 0; i < toolList.count; ++i)
                                (toolList.itemAt(i) as QQC2.CheckBox).checked = false;
                        }
                    }
                    GridLayout {
                        Layout.fillWidth: true
                        columns: 2
                        Repeater {
                            id: toolList
                            model: installer.tool_names.split("\n")
                            delegate: QQC2.CheckBox {
                                required property string modelData
                                Layout.fillWidth: true
                                text: modelData
                                checked: true
                            }
                        }
                    }
                }
            }

            Section {
                title: "USB drive"
                contentItem: ColumnLayout {
                    spacing: Kirigami.Units.smallSpacing
                    Muted {
                        text: "Only writable USB disks are shown. Existing contents will be erased."
                    }
                    RowLayout {
                        Layout.fillWidth: true
                        QQC2.ComboBox {
                            id: diskBox
                            Layout.fillWidth: true
                            model: installer.devices ? installer.devices.split("\n") : []
                            Accessible.name: "USB drive"
                        }
                        Button {
                            text: "Refresh"
                            onClicked: installer.refresh()
                        }
                    }
                }
            }

            Section {
                title: "Status"
                visible: installer.status.length > 0
                contentItem: Muted {
                    text: installer.status
                }
            }

            RowLayout {
                Layout.fillWidth: true
                Muted {
                    text: selectedIsoIndexes ? selectedIsoIndexes.split(",").length + " image(s) selected" : "No images selected"
                }
                Button {
                    text: "Create USB"
                    primary: true
                    enabled: diskBox.currentIndex >= 0 && window.selectedIsoIndexes.length > 0
                    onClicked: eraseDialog.open()
                }
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
