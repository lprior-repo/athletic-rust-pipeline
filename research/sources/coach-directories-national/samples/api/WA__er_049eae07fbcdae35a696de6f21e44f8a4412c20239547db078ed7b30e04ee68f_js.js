"use strict";

import Version from "models/version_number"
import configuration from "models/configuration"

/*
 * Version Check Controller can check the server version against the client version
 * and decide if the client needs prompted to refresh their browser.
 *
 * Usage:
 *
 * // VersionChecker will automatically poll the server for the latest version every hour.
 * vc = new VersionChecker()
 * vc.poll()
 *
 * // Or to explicitly check the version:
 * vc.checkVersion()
 *
 */
class VersionChecker {
  static notificationId = "javascript-outdated-notification"

  constructor() {
    this.checkerInterval = null
  }

  poll() {
    const interval = 3600000 // 1 hour
    this.checkerInterval = setInterval(this.checkVersion.bind(this), interval)
  }

  stopPolling() {
    clearInterval(this.checkerInterval)
  }

  async checkVersion() {
    await configuration.updateIfNeeded()
    const clientVersionTag = document.querySelector("meta[name=version]")
    if (!clientVersionTag || !configuration.serverVersion) {
      console.warn("Skipping version check due to missing version information: ", clientVersionTag, configuration.serverVersion)
      return
    }
    const clientVersion = new Version(clientVersionTag.content)
    const serverVersion = new Version(configuration.serverVersion)

    if (!clientVersion.valid || !serverVersion.valid) {
      console.warn("Skipping version check due to invalid version number: ", clientVersion, serverVersion)
      return
    }

    const outOfDateNess = serverVersion.distanceOutOfDate(clientVersion)

    if (outOfDateNess >= 5) {
      clearInterval(this.checkerInterval)
      this.showUpdateNotification()
      this.stopPolling()
    } else if (outOfDateNess > 0) {
      console.debug("Client is out of date, but only a little")
    }
  }

  showUpdateNotification() {
    if (document.querySelector(`#${VersionChecker.notificationId}`)) {
      return
    }

    const flashContainer = document.querySelector("#flash-content")
    document.querySelector("#body-wrapper").insertBefore(this.notification(), flashContainer)
  }

  notification() {
    const container = document.createElement("div")
    container.id = VersionChecker.notificationId
    container.classList.add("container")

    const notification = document.createElement("div")
    notification.classList.add("notification")
    container.appendChild(notification)

    notification.innerHTML = `
      <span class="icon">
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="none">
          <rect width="20" height="20" rx="10" fill="currentColor"/>
          <path d="M9.54413 5.09585C9.68802 5.03249 9.84355 4.99983 10.0008 4.99996C10.1579 5 10.3133 5.03279 10.457 5.09623C10.6008 5.15967 10.7297 5.25238 10.8357 5.36844C10.9416 5.48449 11.0221 5.62135 11.0722 5.77028C11.1223 5.91922 11.1408 6.07695 11.1265 6.23342L10.6757 11.1945C10.6582 11.3613 10.5795 11.5157 10.4549 11.6279C10.3303 11.7402 10.1685 11.8023 10.0008 11.8023C9.83307 11.8023 9.6713 11.7402 9.54667 11.6279C9.42203 11.5157 9.34337 11.3613 9.32584 11.1945L8.87382 6.23342C8.85953 6.07685 8.87805 5.91901 8.9282 5.76999C8.97835 5.62097 9.05903 5.48406 9.16508 5.36798C9.27114 5.2519 9.40023 5.15922 9.54413 5.09585Z" fill="white"/>
          <path d="M11.0811 13.9197C11.0811 14.5163 10.5975 15 10.0008 15C9.4042 15 8.92053 14.5163 8.92053 13.9197C8.92053 13.323 9.4042 12.8394 10.0008 12.8394C10.5975 12.8394 11.0811 13.323 11.0811 13.9197Z" fill="white"/>
        </svg>
      </span>
      <div class="interaction">
        <div class="copy">
          FinalForms has been updated with new features and improvements. Please refresh your browser to continue using the latest version.
        </div>
        <button class="btn btn-default update-button">Refresh Browser</button>
      </div>
      <span class="dismiss-button">
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg">
        <g id="X">
        <path id="Icon" d="M12 4L4 12M4 4L12 12" stroke="#58595B" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
        </g>
        </svg>
      </span>
    `

    const updateButton = notification.querySelector(".update-button")
    updateButton.addEventListener("click", this.reload)

    const dismissButton = notification.querySelector(".dismiss-button")
    dismissButton.addEventListener("click", () => {
      container.remove()
    })

    return container
  }

  reload() {
    window.location.reload()
  }
}

const versionChecker = new VersionChecker()
export default versionChecker
versionChecker.poll()
// Trigger checkVersion when the page becomes visible
document.addEventListener("visibilitychange", () => {
  if (document.visibilityState === "visible") {
    versionChecker.checkVersion();
  }
});
window.addEventListener("focus", versionChecker.checkVersion.bind(versionChecker));
