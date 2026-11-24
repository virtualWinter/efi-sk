import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

// installer is injected by the Rust QML engine at runtime.
// qmllint disable unqualified
Kirigami.ApplicationWindow {
    id: window
    visible: true
    width: 800
    height: 680
    minimumWidth: 620
    minimumHeight: 560
    title: "EFI Swiss Knife"

    property string selectedIsoIndexes: ""

    function toggleIso(index, checked) {
        var values = selectedIsoIndexes ? selectedIsoIndexes.split(",") : []
        var position = values.indexOf(String(index))
        if (checked && position < 0)
            values.push(String(index))
        else if (!checked && position >= 0)
            values.splice(position, 1)
        selectedIsoIndexes = values.join(",")
    }

    pageStack.initialPage: Kirigami.Page {
        title: "EFI Swiss Knife"

        QQC2.ScrollView {
            anchors.fill: parent
            contentWidth: availableWidth
            clip: true

            ColumnLayout {
                width: Math.min(parent.width, 700)
                anchors.horizontalCenter: parent.horizontalCenter
                spacing: Kirigami.Units.largeSpacing

                Kirigami.Heading {
                    text: "Create a bootable USB"
                    level: 1
                    Layout.fillWidth: true
                }
                QQC2.Label {
                    text: "Choose the images you want to carry with you, then install them on a USB drive."
                    color: Kirigami.Theme.disabledTextColor
                    Layout.fillWidth: true
                    wrapMode: Text.Wrap
                }

                Kirigami.Card {
                    Layout.fillWidth: true
                    banner.title: "Boot images"
                    banner.titleIcon: "media-optical"
                    contentItem: ColumnLayout {
                        spacing: Kirigami.Units.smallSpacing
                        QQC2.Label {
                            text: "Select one or more images. Downloaded images are resumed and verified."
                            color: Kirigami.Theme.disabledTextColor
                            Layout.fillWidth: true
                            wrapMode: Text.Wrap
                        }
                        ListView {
                            id: imageList
                            Layout.fillWidth: true
                            Layout.preferredHeight: Math.min(contentHeight, 220)
                            implicitHeight: contentHeight
                            clip: true
                            model: installer.iso_names.split("\n")
                            spacing: 1
                            delegate: QQC2.ItemDelegate {
                                width: imageList.width
                                contentItem: RowLayout {
                                    spacing: Kirigami.Units.smallSpacing
                                    Kirigami.Icon {
                                        source: installer.iso_icons.split("\n")[index]
                                        implicitWidth: Kirigami.Units.iconSizes.medium
                                        implicitHeight: Kirigami.Units.iconSizes.medium
                                        fallback: "application-x-iso9660-image"
                                    }
                                    QQC2.CheckBox {
                                        checked: window.selectedIsoIndexes.split(",").indexOf(String(index)) >= 0
                                        onCheckedChanged: if (pressed) window.toggleIso(index, checked)
                                        Accessible.name: "Include " + modelData
                                    }
                                    QQC2.Label {
                                        text: modelData
                                        Layout.fillWidth: true
                                        elide: Text.ElideMiddle
                                    }
                                    QQC2.Button {
                                        text: "Download"
                                        icon.name: "download"
                                        flat: true
                                        onClicked: installer.download_iso(index)
                                    }
                                }
                            }
                        }
                    }
                }

                Kirigami.Card {
                    Layout.fillWidth: true
                    banner.title: "Optional EFI tools"
                    contentItem: ColumnLayout {
                        QQC2.Label {
                            text: "EFI release version"
                            visible: installer.release_mode
                        }
                        QQC2.TextField {
                            id: efiVersion
                            visible: installer.release_mode
                            Layout.fillWidth: true
                            placeholderText: "latest (or a tag, e.g. esk-0.1.1)"
                            Accessible.name: "EFI release version"
                        }
                        QQC2.Label {
                            text: "Choose which tools to install. The boot picker is always included."
                            Layout.fillWidth: true
                            wrapMode: Text.Wrap
                        }
                        QQC2.Button {
                            text: "Deselect all tools"
                            onClicked: {
                                for (let i = 0; i < toolList.count; ++i)
                                    (toolList.itemAt(i) as QQC2.CheckBox).checked = false
                            }
                        }
                        Repeater {
                            id: toolList
                            model: installer.tool_names.split("\n")
                            delegate: QQC2.CheckBox {
                                required property string modelData
                                text: modelData
                                checked: true
                            }
                        }
                    }
                }

                Kirigami.Card {
                    Layout.fillWidth: true
                    banner.title: "USB drive"
                    banner.titleIcon: "drive-removable-media"
                    contentItem: ColumnLayout {
                        spacing: Kirigami.Units.smallSpacing
                        QQC2.Label {
                            text: "Only writable USB disks are shown. Existing contents will be erased."
                            color: Kirigami.Theme.disabledTextColor
                            Layout.fillWidth: true
                            wrapMode: Text.Wrap
                        }
                        RowLayout {
                            Layout.fillWidth: true
                            QQC2.ComboBox {
                                id: diskBox
                                Layout.fillWidth: true
                                model: installer.devices ? installer.devices.split("\n") : []
                                Accessible.name: "USB drive"
                            }
                            QQC2.Button {
                                text: "Refresh"
                                icon.name: "view-refresh"
                                onClicked: installer.refresh()
                            }
                        }
                    }
                }

                Kirigami.InlineMessage {
                    Layout.fillWidth: true
                    type: Kirigami.MessageType.Information
                    text: installer.status
                    visible: installer.status.length > 0
                }

                RowLayout {
                    Layout.fillWidth: true
                    QQC2.Label {
                        text: selectedIsoIndexes ? selectedIsoIndexes.split(",").length + " image(s) selected" : "No images selected"
                        color: Kirigami.Theme.disabledTextColor
                        Layout.fillWidth: true
                    }
                    QQC2.Button {
                        text: "Create USB"
                        icon.name: "dialog-ok-apply"
                        enabled: diskBox.currentIndex >= 0 && window.selectedIsoIndexes.length > 0
                        onClicked: eraseDialog.open()
                    }
                }
            }
        }
    }

    Kirigami.PromptDialog {
        id: eraseDialog
        title: "Erase USB drive?"
        subtitle: diskBox.currentText + "\n\nSelected images and tools will be installed. Existing data will be permanently erased."
        standardButtons: QQC2.Dialog.Cancel | QQC2.Dialog.Ok
        onAccepted: {
            let excluded = []
            for (let i = 0; i < toolList.count; ++i) {
                if (!(toolList.itemAt(i) as QQC2.CheckBox).checked)
                    excluded.push(i)
            }
            installer.create_usb(window.selectedIsoIndexes, diskBox.currentIndex, excluded.join(","), efiVersion.text)
        }
    }
}
