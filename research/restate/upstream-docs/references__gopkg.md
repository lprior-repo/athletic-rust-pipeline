

<!DOCTYPE html>
<html lang="en" data-layout="responsive" data-local="">
  <head>
    
    <script>
      window.addEventListener('error', window.__err=function f(e){f.p=f.p||[];f.p.push(e)});
    </script>
    <script>
      (function() {
        const theme = document.cookie.match(/prefers-color-scheme=(light|dark|auto)/)?.[1]
        if (theme) {
          document.querySelector('html').setAttribute('data-theme', theme);
        }
      }())
    </script>
    <meta charset="utf-8">
    <meta http-equiv="X-UA-Compatible" content="IE=edge">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    
    
    <meta class="js-gtmID" data-gtmid="GTM-W8MVQXG">
    <link rel="shortcut icon" href="/static/shared/icon/favicon.ico">
    
  
    <link rel="canonical" href="https://pkg.go.dev/github.com/restatedev/sdk-go">
  

    <link href="/static/frontend/frontend.min.css?version=prod-frontend-00254-flh" rel="stylesheet">
    
    <link rel="search" type="application/opensearchdescription+xml" href="/opensearch.xml" title="Go Packages">
    
    
  <title>restate package - github.com/restatedev/sdk-go - Go Packages</title>

    
  <link href="/static/frontend/unit/unit.min.css?version=prod-frontend-00254-flh" rel="stylesheet">
  
  <link href="/static/frontend/unit/main/main.min.css?version=prod-frontend-00254-flh" rel="stylesheet">


  </head>
  <body>
    
    <script>
      function loadScript(src, mod = true) {
        let s = document.createElement('script');
        s.src = src;
        if (mod) {
          s.type = 'module';
          s.async = true;
          s.defer = true
        }
        document.head.appendChild(s);
      }
      loadScript("/third_party/dialog-polyfill/dialog-polyfill.js", false)
      loadScript("/static/frontend/frontend.js");
    </script>
    
  <header class="go-Header go-Header--full js-siteHeader">
    <div class="go-Header-inner go-Header-inner--dark">
      <nav class="go-Header-nav">
        <a href="https://go.dev/" class="js-headerLogo" data-gtmc="nav link"
            data-test-id="go-header-logo-link" role="heading" aria-level="1">
          <img class="go-Header-logo" src="/static/shared/logo/go-white.svg" alt="Go">
        </a>
         <div class="skip-navigation-wrapper">
            <a class="skip-to-content-link" aria-label="Skip to main content" href="#main-content"> Skip to Main Content </a>
          </div>
        <div class="go-Header-rightContent">
          
<div class="go-SearchForm js-searchForm">
  <form
    class="go-InputGroup go-ShortcutKey go-SearchForm-form"
    action="/search"
    data-shortcut="/"
    data-shortcut-alt="search"
    data-gtmc="search form"
    aria-label="Search for a package"
    role="search"
  >
    <input name="q" class="go-Input js-searchFocus" aria-label="Search for a package" type="search"
        autocapitalize="off" autocomplete="off" autocorrect="off" spellcheck="false"
        placeholder="Search packages or symbols"
        value="" />
    <input name="m" value="" hidden>
    <button class="go-Button go-Button--inverted" aria-label="Submit search">
      <img
        class="go-Icon"
        height="24"
        width="24"
        src="/static/shared/icon/search_gm_grey_24dp.svg"
        alt=""
      />
    </button>
  </form>
  <button class="go-SearchForm-expandSearch js-expandSearch" data-gtmc="nav button"
      aria-label="Open search" data-test-id="expand-search">
    <img class="go-Icon go-Icon--inverted" height="24" width="24"
        src="/static/shared/icon/search_gm_grey_24dp.svg" alt="">

  </button>
</div>

          <ul class="go-Header-menu">
            <li class="go-Header-menuItem">
              <a class="js-desktop-menu-hover" href="#" data-gtmc="nav link">
                Why Go
                <img class="go-Icon" height="24" width="24" src="/static/shared/icon/arrow_drop_down_gm_grey_24dp.svg" alt="submenu dropdown icon">
              </a>
              <ul class="go-Header-submenu go-Header-submenu--why js-desktop-submenu-hover" aria-label="submenu">
                  <li class="go-Header-submenuItem">
                    <div>
                      <a href="https://go.dev/solutions/case-studies">
                        <span>Case Studies</span>
                      </a>
                    </div>
                    <p>Common problems companies solve with Go</p>
                  </li>
                  <li class="go-Header-submenuItem">
                    <div>
                      <a href="https://go.dev/solutions/use-cases">
                        <span>Use Cases</span>
                      </a>
                    </div>
                    <p>Stories about how and why companies use Go</p>
                  </li>
                  <li class="go-Header-submenuItem">
                    <div>
                      <a href="https://go.dev/security/">
                        <span>Security</span>
                      </a>
                    </div>
                    <p>How Go can help keep you secure by default</p>
                  </li>
              </ul>
            </li>
            <li class="go-Header-menuItem">
              <a href="https://go.dev/learn/" data-gtmc="nav link">Learn</a>
            </li>
            <li class="go-Header-menuItem">
              <a class="js-desktop-menu-hover" href="#" data-gtmc="nav link">
                Docs
                <img class="go-Icon" height="24" width="24" src="/static/shared/icon/arrow_drop_down_gm_grey_24dp.svg" alt="submenu dropdown icon">
              </a>
              <ul class="go-Header-submenu go-Header-submenu--docs js-desktop-submenu-hover" aria-label="submenu">
                <li class="go-Header-submenuItem">
                  <div>
                    <a href="https://go.dev/doc/effective_go">
                      <span>Effective Go</span>
                    </a>
                  </div>
                  <p>Tips for writing clear, performant, and idiomatic Go code</p>
                </li>
                <li class="go-Header-submenuItem">
                  <div>
                    <a href="https://go.dev/doc/">
                      <span>Go User Manual</span>
                    </a>
                  </div>
                  <p>A complete introduction to building software with Go</p>
                </li>
                <li class="go-Header-submenuItem">
                  <div>
                    <a href="https://pkg.go.dev/std">
                      <span>Standard library</span>
                    </a>
                  </div>
                  <p>Reference documentation for Go's standard library</p>
                </li>
                <li class="go-Header-submenuItem">
                  <div>
                    <a href="https://go.dev/doc/devel/release">
                      <span>Release Notes</span>
                    </a>
                  </div>
                  <p>Learn what's new in each Go release</p>
                </li>
                <li class="go-Header-submenuItem">
                  <div>
                    <a href="/api">
                      <span>API</span>
                    </a>
                  </div>
                  <p>Reference documentation for the pkg.go.dev API</p>
                </li>
              </ul>
            </li>
            <li class="go-Header-menuItem go-Header-menuItem--active">
              <a href="/" data-gtmc="nav link">Packages</a>
            </li>
            <li class="go-Header-menuItem">
              <a class="js-desktop-menu-hover" href="#" data-gtmc="nav link">
                Community
                <img class="go-Icon" height="24" width="24" src="/static/shared/icon/arrow_drop_down_gm_grey_24dp.svg" alt="submenu dropdown icon">
              </a>
              <ul class="go-Header-submenu go-Header-submenu--community js-desktop-submenu-hover" aria-label="submenu">
                <li class="go-Header-submenuItem">
                  <div>
                    <a href="https://go.dev/talks/">
                      <span>Recorded Talks</span>
                    </a>
                  </div>
                  <p>Videos from prior events</p>
                </li>
                <li class="go-Header-submenuItem">
                  <div>
                    <a href="https://www.meetup.com/pro/go">
                      <span>Meetups</span>
                      <i class="material-icons">
                        <img class="go-Icon" height="24" width="24"
                            src="/static/shared/icon/launch_gm_grey_24dp.svg" alt="">
                      </i>
                    </a>
                  </div>
                  <p>Meet other local Go developers</p>
                </li>
                <li class="go-Header-submenuItem">
                  <div>
                    <a href="https://github.com/golang/go/wiki/Conferences">
                      <span>Conferences</span>
                      <i class="material-icons">
                        <img class="go-Icon" height="24" width="24"
                            src="/static/shared/icon/launch_gm_grey_24dp.svg" alt="">
                      </i>
                    </a>
                  </div>
                  <p>Learn and network with Go developers from around the world</p>
                </li>
                <li class="go-Header-submenuItem">
                  <div>
                    <a href="https://go.dev/blog">
                      <span>Go blog</span>
                    </a>
                  </div>
                  <p>The Go project's official blog.</p>
                </li>
                <li class="go-Header-submenuItem">
                  <div>
                    <a href="https://go.dev/help">
                      <span>Go project</span>
                    </a>
                  </div>
                  <p>Get help and stay informed from Go</p>
                </li>
                <li class="go-Header-submenuItem">
                  <div>
                    Get connected
                  </div>
                  <p></p>
                  <div class="go-Header-socialIcons">
                      <a
                        class="go-Header-socialIcon"
                        aria-label="Get connected with google-groups (Opens in new window)"
                        title="Get connected with google-groups (Opens in new window)"
                        href="https://groups.google.com/g/golang-nuts">
                        <img src="/static/shared/logo/social/google-groups.svg" />
                      </a>
                      <a
                        class="go-Header-socialIcon"
                        aria-label="Get connected with github (Opens in new window)"
                        title="Get connected with github (Opens in new window)"
                        href="https://github.com/golang">
                        <img src="/static/shared/logo/social/github.svg" />
                      </a>
                      <a
                        class="go-Header-socialIcon"
                        aria-label="Get connected with twitter (Opens in new window)"
                        title="Get connected with twitter (Opens in new window)"
                        href="https://twitter.com/golang">
                        <img src="/static/shared/logo/social/twitter.svg" />
                      </a>
                      <a
                        class="go-Header-socialIcon"
                        aria-label="Get connected with reddit (Opens in new window)"
                        title="Get connected with reddit (Opens in new window)"
                        href="https://www.reddit.com/r/golang/">
                        <img src="/static/shared/logo/social/reddit.svg" />
                      </a>
                      <a
                        class="go-Header-socialIcon"
                        aria-label="Get connected with slack (Opens in new window)"
                        title="Get connected with slack (Opens in new window)"
                        href="https://invite.slack.golangbridge.org/">
                        <img src="/static/shared/logo/social/slack.svg" />
                      </a>
                      <a
                        class="go-Header-socialIcon"
                        aria-label="Get connected with stack-overflow (Opens in new window)"
                        title=""
                        href="https://stackoverflow.com/collectives/go">
                        <img src="/static/shared/logo/social/stack-overflow.svg" />
                      </a>
                  </div>
                </li>
              </ul>
            </li>
          </ul>
          <button class="go-Header-navOpen js-headerMenuButton go-Header-navOpen--white" data-gtmc="nav button" aria-label="Open navigation">
          </button>
        </div>
      </nav>
    </div>
  </header>
  <aside class="go-NavigationDrawer js-header">
    <nav class="go-NavigationDrawer-nav">
      <div class="go-NavigationDrawer-header">
        <a href="https://go.dev/">
          <img class="go-NavigationDrawer-logo" src="/static/shared/logo/go-blue.svg" alt="Go.">
        </a>
      </div>
      <ul class="go-NavigationDrawer-list">
          <li class="go-NavigationDrawer-listItem js-mobile-subnav-trigger go-NavigationDrawer-hasSubnav">
            <a href="#">
              <span>Why Go</span>
              <i class="material-icons">
                <img class="go-Icon" height="24" width="24"
                  src="/static/shared/icon/navigate_next_gm_grey_24dp.svg" alt="">
              </i>
            </a>

            <div class="go-NavigationDrawer go-NavigationDrawer-submenuItem">
              <div class="go-NavigationDrawer-nav">
                <div class="go-NavigationDrawer-header">
                  <a href="#">
                    <i class="material-icons">
                      <img class="go-Icon" height="24" width="24"
                        src="/static/shared/icon/navigate_before_gm_grey_24dp.svg" alt="">
                      </i>
                      Why Go
                  </a>
                </div>
                <ul class="go-NavigationDrawer-list">
                  <li class="go-NavigationDrawer-listItem">
                    <a href="https://go.dev/solutions/case-studies">
                      Case Studies
                    </a>
                  </li>
                  <li class="go-NavigationDrawer-listItem">
                    <a href="https://go.dev/solutions/use-cases">
                      Use Cases
                    </a>
                  </li>
                  <li class="go-NavigationDrawer-listItem">
                    <a href="https://go.dev/security/">
                      Security
                    </a>
                  </li>
                </ul>
              </div>
            </div>
          </li>
          <li class="go-NavigationDrawer-listItem">
            <a href="https://go.dev/learn/">Learn</a>
          </li>
          <li class="go-NavigationDrawer-listItem js-mobile-subnav-trigger go-NavigationDrawer-hasSubnav">
            <a href="#">
              <span>Docs</span>
              <i class="material-icons">
                <img class="go-Icon" height="24" width="24"
                  src="/static/shared/icon/navigate_next_gm_grey_24dp.svg" alt="">
              </i>
            </a>

            <div class="go-NavigationDrawer go-NavigationDrawer-submenuItem">
              <div class="go-NavigationDrawer-nav">
                <div class="go-NavigationDrawer-header">
                  <a href="#"><i class="material-icons">
                    <img class="go-Icon" height="24" width="24"
                      src="/static/shared/icon/navigate_before_gm_grey_24dp.svg" alt="">
                    </i>
                    Docs
                  </a>
                </div>
                <ul class="go-NavigationDrawer-list">
                  <li class="go-NavigationDrawer-listItem">
                    <a href="https://go.dev/doc/effective_go">
                      Effective Go
                    </a>
                  </li>
                  <li class="go-NavigationDrawer-listItem">
                    <a href="https://go.dev/doc/">
                      Go User Manual
                    </a>
                  </li>
                  <li class="go-NavigationDrawer-listItem">
                    <a href="https://pkg.go.dev/std">
                      Standard library
                    </a>
                  </li>
                  <li class="go-NavigationDrawer-listItem">
                    <a href="https://go.dev/doc/devel/release">
                      Release Notes
                    </a>
                  </li>
                  <li class="go-NavigationDrawer-listItem">
                    <a href="/api">
                      API
                    </a>
                  </li>
                </ul>
              </div>
            </div>
          </li>
          <li class="go-NavigationDrawer-listItem go-NavigationDrawer-listItem--active">
            <a href="/">Packages</a>
          </li>
          <li class="go-NavigationDrawer-listItem js-mobile-subnav-trigger go-NavigationDrawer-hasSubnav">
            <a href="#">
              <span>Community</span>
              <i class="material-icons">
                <img class="go-Icon" height="24" width="24"
                  src="/static/shared/icon/navigate_next_gm_grey_24dp.svg" alt="">
              </i>
            </a>
            <div class="go-NavigationDrawer go-NavigationDrawer-submenuItem">
              <div class="go-NavigationDrawer-nav">
                <div class="go-NavigationDrawer-header">
                  <a href="#">
                    <i class="material-icons">
                      <img class="go-Icon" height="24" width="24"
                        src="/static/shared/icon/navigate_before_gm_grey_24dp.svg" alt="">
                    </i>
                    Community
                  </a>
                </div>
                <ul class="go-NavigationDrawer-list">
                  <li class="go-NavigationDrawer-listItem">
                    <a href="https://go.dev/talks/">
                      Recorded Talks
                    </a>
                  </li>
                  <li class="go-NavigationDrawer-listItem">
                    <a href="https://www.meetup.com/pro/go">
                      Meetups
                      <i class="material-icons">
                      <img class="go-Icon" height="24" width="24"
                          src="/static/shared/icon/launch_gm_grey_24dp.svg" alt="">
                      </i>
                    </a>
                  </li>
                  <li class="go-NavigationDrawer-listItem">
                    <a href="https://github.com/golang/go/wiki/Conferences">
                      Conferences
                      <i class="material-icons">
                        <img class="go-Icon" height="24" width="24" src="/static/shared/icon/launch_gm_grey_24dp.svg" alt="">
                      </i>
                    </a>
                  </li>
                  <li class="go-NavigationDrawer-listItem">
                    <a href="https://go.dev/blog">
                      Go blog
                    </a>
                  </li>
                  <li class="go-NavigationDrawer-listItem">
                    <a href="https://go.dev/help">
                      Go project
                    </a>
                  </li>
                  <li class="go-NavigationDrawer-listItem">
                    <div>Get connected</div>
                    <div class="go-Header-socialIcons">
                        <a class="go-Header-socialIcon" href="https://groups.google.com/g/golang-nuts"><img src="/static/shared/logo/social/google-groups.svg" /></a>
                        <a class="go-Header-socialIcon" href="https://github.com/golang"><img src="/static/shared/logo/social/github.svg" /></a>
                        <a class="go-Header-socialIcon" href="https://twitter.com/golang"><img src="/static/shared/logo/social/twitter.svg" /></a>
                        <a class="go-Header-socialIcon" href="https://www.reddit.com/r/golang/"><img src="/static/shared/logo/social/reddit.svg" /></a>
                        <a class="go-Header-socialIcon" href="https://invite.slack.golangbridge.org/"><img src="/static/shared/logo/social/slack.svg" /></a>
                        <a class="go-Header-socialIcon" href="https://stackoverflow.com/collectives/go"><img src="/static/shared/logo/social/stack-overflow.svg" /></a>
                    </div>
                  </li>
                </ul>
              </div>
            </div>
          </li>
      </ul>
    </nav>
  </aside>
  <div class="go-NavigationDrawer-scrim js-scrim" role="presentation"></div>

    
  <main class="go-Main" id="main-content">
    <div class="go-Main-banner" role="alert"></div>
    <header class="go-Main-header js-mainHeader">
  
  
  <nav class="go-Main-headerBreadcrumb go-Breadcrumb" aria-label="Breadcrumb" data-test-id="UnitHeader-breadcrumb">
    <ol>
      
        
          <li data-test-id="UnitHeader-breadcrumbItem">
            <a href="/" data-gtmc="breadcrumb link">Discover Packages</a>
          </li>
        
        <li>
          <a href="/github.com/restatedev/sdk-go@v1.1.0" data-gtmc="breadcrumb link" aria-current="location"
              data-test-id="UnitHeader-breadcrumbCurrent">
            github.com/restatedev/sdk-go
          </a>
          
            <button
              class="go-Button go-Button--inline go-Clipboard js-clipboard"
              title="Copy path to clipboard.&#10;&#10;github.com/restatedev/sdk-go"
              aria-label="Copy Path to Clipboard"
              data-to-copy="github.com/restatedev/sdk-go"
              data-gtmc="breadcrumbs button"
            >
              <img
                class="go-Icon go-Icon--accented"
                height="24"
                width="24"
                src="/static/shared/icon/content_copy_gm_grey_24dp.svg"
                alt=""
              >
            </button>
          
        
      </li>
    </ol>
  </nav>

  <div class="go-Main-headerContent">
    
  <div class="go-Main-headerTitle js-stickyHeader">
    <a class="go-Main-headerLogo" href="https://go.dev/" aria-hidden="true" tabindex="-1" data-gtmc="header link" aria-label="Link to Go Homepage">
      <img height="78" width="207" src="/static/shared/logo/go-blue.svg" alt="Go">
    </a>
    <h1 class="UnitHeader-titleHeading" data-test-id="UnitHeader-title">restate</h1>
    
      <span class="go-Chip go-Chip--inverted">package</span>
    
      <span class="go-Chip go-Chip--inverted">module</span>
    
    
      
        <button
          class="go-Button go-Button--inline go-Clipboard js-clipboard"
          title="Copy path to clipboard.&#10;&#10;github.com/restatedev/sdk-go"
          aria-label="Copy Path to Clipboard"
          data-to-copy="github.com/restatedev/sdk-go"
          data-gtmc="title button"
          tabindex="-1"
        >
          <img
            class="go-Icon go-Icon--accented"
            height="24"
            width="24"
            src="/static/shared/icon/content_copy_gm_grey_24dp.svg"
            alt=""
          />
        </button>
      
    
  </div>

    
      
  <div class="go-Main-headerDetails">
    
      
  <span class="go-Main-headerDetailItem" data-test-id="UnitHeader-version">
    <a href="?tab=versions" aria-label="Version: v1.1.0" 
    data-gtmc="header link" aria-describedby="version-description">
      <span class="go-textSubtle" aria-hidden="true">Version: </span>
        v1.1.0
    </a>
    <div class="screen-reader-only" id="version-description" hidden>
      Opens a new window with list of versions in this module.
    </div>
    
    <span class="DetailsHeader-badge--latest" data-test-id="UnitHeader-minorVersionBanner">
      <span class="go-Chip DetailsHeader-span--latest">Latest</span>
      <span class="go-Chip DetailsHeader-span--notAtLatest">
        Latest
        
  <details class="go-Tooltip js-tooltip" data-gtmc="tooltip">
    <summary>
      <img class="go-Icon go-Icon--inverted" height="24" width="24" src="/static/shared/icon/alert_gm_grey_24dp.svg" alt="Warning">
    </summary>
    <p>This package is not in the latest version of its module.</p>
  </details>

      </span>
      <a href="/github.com/restatedev/sdk-go" aria-label="Go to Latest Version" data-gtmc="header link">
        <span class="go-Chip go-Chip--alert DetailsHeader-span--goToLatest">Go to latest</span>
      </a>
    </span>
  </span>

      
  <span class="go-Main-headerDetailItem" data-test-id="UnitHeader-commitTime">
    Published: Sep 22, 2026
  </span>

      
  <span class="go-Main-headerDetailItem" data-test-id="UnitHeader-licenses">
    License: <a href="/github.com/restatedev/sdk-go?tab=licenses" data-test-id="UnitHeader-license" 
        data-gtmc="header link" aria-describedby="license-description">MIT</a>
      
    
  </span>
  <div class="screen-reader-only" id="license-description" hidden>
    Opens a new window with license information.
  </div>

      
        
  <span class="go-Main-headerDetailItem" data-test-id="UnitHeader-imports">
    <a href="/github.com/restatedev/sdk-go?tab=imports" aria-label="Imports: 15"
        data-gtmc="header link" aria-describedby="imports-description">
      <span class="go-textSubtle">Imports: </span>15
    </a>
  </span>
  <div class="screen-reader-only" id="imports-description" hidden>
    Opens a new window with list of imports.
  </div>

        
  <span class="go-Main-headerDetailItem" data-test-id="UnitHeader-importedby">
    <a href="/github.com/restatedev/sdk-go?tab=importedby" aria-label="Imported By: 63"
        data-gtmc="header link" aria-describedby="importedby-description">
       <span class="go-textSubtle">Imported by: </span>63
    </a>
  </span>
  <div class="screen-reader-only" id="importedby-description" hidden>
    Opens a new window with list of known importers.
  </div>

      
    
  </div>
  
  <div class="UnitHeader-overflowContainer">
    <svg class="UnitHeader-overflowImage" xmlns="http://www.w3.org/2000/svg" height="24" viewBox="0 0 24 24" width="24">
      <path d="M0 0h24v24H0z" fill="none"/>
      <path d="M12 8c1.1 0 2-.9 2-2s-.9-2-2-2-2 .9-2 2 .9 2 2 2zm0 2c-1.1 0-2 .9-2 2s.9 2 2 2 2-.9 2-2-.9-2-2-2zm0 6c-1.1 0-2 .9-2 2s.9 2 2 2 2-.9 2-2-.9-2-2-2z"/>
    </svg>
    <select class="UnitHeader-overflowSelect js-selectNav" tabindex="-1">
      <option value="/">Main</option>
      <option value="/github.com/restatedev/sdk-go?tab=versions">
        Versions
      </option>
      <option value="/github.com/restatedev/sdk-go?tab=licenses">
        Licenses
      </option>
      
        <option value="/github.com/restatedev/sdk-go?tab=imports">
          Imports
        </option>
        <option value="/github.com/restatedev/sdk-go?tab=importedby">
          Imported By
        </option>
      
    </select>
  </div>


    
  </div>

</header>
    
      <aside class="go-Main-aside  js-mainAside">
  
  <div class="UnitMeta">
    <h2 class="go-textLabel">Details</h2>
    
  <ul class="UnitMeta-details">
    <li>
      <details class="go-Tooltip js-tooltip" data-gtmc="tooltip">
        <summary class="go-textSubtle">
          
  <img class="go-Icon go-Icon--accented"
    tabindex="0"
    role="button"src="/static/shared/icon/check_circle_gm_grey_24dp.svg" alt="checked" aria-label="Valid file, toggle tooltip"height="24" width="24">

          Valid <a href="https://github.com/restatedev/sdk-go/tree/v1.1.0/go.mod" target="_blank" rel="noopener">go.mod</a> file
          <img class="go-Icon" role="button" tabindex="0" src="/static/shared/icon/help_gm_grey_24dp.svg" alt="" aria-label="Toggle go.mod validity tooltip" height="24" width="24">
        </summary>
        <p aria-live="polite" role="tooltip">
          The Go module system was introduced in Go 1.11 and is the official dependency management
          solution for Go.
        </p>
      </details>
    </li>
    <li>
      <details class="go-Tooltip js-tooltip" data-gtmc="tooltip">
        <summary class="go-textSubtle">
          
  <img class="go-Icon go-Icon--accented"
    tabindex="0"
    role="button"src="/static/shared/icon/check_circle_gm_grey_24dp.svg" alt="checked" aria-label="Valid file, toggle tooltip"height="24" width="24">

          Redistributable license
          <img class="go-Icon" role="button" tabindex="0" src="/static/shared/icon/help_gm_grey_24dp.svg" alt="" aria-label="Toggle redistributable help tooltip" height="24" width="24">
        </summary>
        <p aria-live="polite" role="tooltip">
          Redistributable licenses place minimal restrictions on how software can be used,
          modified, and redistributed.
        </p>
      </details>
    </li>
    <li>
      <details class="go-Tooltip js-tooltip" data-gtmc="tooltip">
        <summary class="go-textSubtle">
          
  <img class="go-Icon go-Icon--accented"
    tabindex="0"
    role="button"src="/static/shared/icon/check_circle_gm_grey_24dp.svg" alt="checked" aria-label="Valid file, toggle tooltip"height="24" width="24">

          Tagged version
          <img class="go-Icon" role="button" tabindex="0" src="/static/shared/icon/help_gm_grey_24dp.svg" alt="" aria-label="Toggle tagged version tooltip" height="24" width="24">
        </summary>
        <p aria-live="polite" role="tooltip">Modules with tagged versions give importers more predictable builds.</p>
      </details>
    </li>
    <li>
      <details class="go-Tooltip js-tooltip" data-gtmc="tooltip">
        <summary class="go-textSubtle">
          
  <img class="go-Icon go-Icon--accented"
    tabindex="0"
    role="button"src="/static/shared/icon/check_circle_gm_grey_24dp.svg" alt="checked" aria-label="Valid file, toggle tooltip"height="24" width="24">

          Stable version
          <img class="go-Icon" role="button" tabindex="0" aria-label="Toggle stable version tooltip" src="/static/shared/icon/help_gm_grey_24dp.svg" alt="" height="24" width="24">
        </summary>
        <p aria-live="polite" role="tooltip">When a project reaches major version v1 it is considered stable.</p>
      </details>
    </li>
    <li class="UnitMeta-detailsLearn">
      <a href="/about#best-practices" data-gtmc="meta link">Learn more about best practices</a>
    </li>
  </ul>

    <h2 class="go-textLabel">Repository</h2>
    <div class="UnitMeta-repo">
      
        <a href="https://github.com/restatedev/sdk-go" title="https://github.com/restatedev/sdk-go" target="_blank" rel="noopener">
          github.com/restatedev/sdk-go
        </a>
      
    </div>
    
      <h2 class="go-textLabel" data-test-id="links-heading">Links</h2>
      <ul class="UnitMeta-links">
        
        
          <li>
            <a href="https://deps.dev/go/github.com%2Frestatedev%2Fsdk-go/v1.1.0" title="View this module on Open Source Insights"
              target="_blank" rel="noopener" data-test-id="meta-link-depsdev">
              <img class="link-Icon" src="/static/shared/icon/depsdev-logo.svg"
                alt="Open Source Insights Logo" />
              Open Source Insights
            </a>
          </li>
        
        
        
  

        
  

        
  

      </ul>
    
  </div>

</aside>
    
    <nav class="go-Main-nav go-Main-nav--sticky js-mainNav" aria-label="Outline">
  <div class="go-Main-navDesktop">
    
  <div class="UnitOutline-jumpTo">
    <button class="UnitOutline-jumpToInput go-ShortcutKey js-jumpToInput"
        aria-controls="jump-to-modal"
        aria-label="Open Jump to Identifier"
        data-shortcut="f"
        data-shortcut-alt="find"
        data-test-id="jump-to-button" data-gtmc="outline button">
      Jump to ...
    </button>
  </div>
  <ul class="go-Tree js-tree" role="tree" aria-label="Outline">
    
      <li class="js-readmeOutline">
        <a href="#section-readme" data-gtmc="outline link">
          README
        </a>
        
  <ul id="readme-outline">
    
      <li>
        <a href="#readme-community" data-gtmc="readme outline link">
          Community
        </a>
         
      </li>
    
      <li>
        <a href="#readme-prerequisites" data-gtmc="readme outline link">
          Prerequisites
        </a>
         
      </li>
    
      <li>
        <a href="#readme-examples" data-gtmc="readme outline link">
          Examples
        </a>
        
          <ul>
            
              <li>
                <a href="#readme-how-to-use-the-example" data-gtmc="readme outline link">
                  How to use the example
                </a>
                 
              </li>
             
          </ul>
         
      </li>
    
      <li>
        <a href="#readme-ingress-sdk" data-gtmc="readme outline link">
          Ingress SDK
        </a>
         
      </li>
    
      <li>
        <a href="#readme-versions" data-gtmc="readme outline link">
          Versions
        </a>
         
      </li>
    
      <li>
        <a href="#readme-contributing" data-gtmc="readme outline link">
          Contributing
        </a>
         
      </li>
     
  </ul>

      </li>
    
    
      <li>
        <a href="#section-documentation" data-gtmc="outline link">
          Documentation
        </a>
        
<ul>
  <li class="DocNav-overview">
      <a href="#pkg-index" data-gtmc="doc outline link">
        Index
      </a>
    </li>
    <li class="DocNav-constants">
      <a href="#pkg-constants" data-gtmc="doc outline link">
        Constants
      </a>
    </li>
    <li class="DocNav-variables">
      <a href="#pkg-variables" data-gtmc="doc outline link">
        Variables
      </a>
    </li>
    <li class="DocNav-functions">
      <a href="#pkg-functions" data-gtmc="doc outline link">
        Functions
      </a>
      
        <ul>
          
            <li>
              <a href="#CancelInvocation" title="CancelInvocation(ctx, invocationId)" data-gtmc="doc outline link">
                CancelInvocation(ctx, invocationId)
              </a>
            </li>
          
            <li>
              <a href="#Clear" title="Clear(ctx, key)" data-gtmc="doc outline link">
                Clear(ctx, key)
              </a>
            </li>
          
            <li>
              <a href="#ClearAll" title="ClearAll(ctx)" data-gtmc="doc outline link">
                ClearAll(ctx)
              </a>
            </li>
          
            <li>
              <a href="#IsRetryableError" title="IsRetryableError(err)" data-gtmc="doc outline link">
                IsRetryableError(err)
              </a>
            </li>
          
            <li>
              <a href="#IsTerminalError" title="IsTerminalError(err)" data-gtmc="doc outline link">
                IsTerminalError(err)
              </a>
            </li>
          
            <li>
              <a href="#Key" title="Key(ctx)" data-gtmc="doc outline link">
                Key(ctx)
              </a>
            </li>
          
            <li>
              <a href="#KillOnMaxAttempts" title="KillOnMaxAttempts()" data-gtmc="doc outline link">
                KillOnMaxAttempts()
              </a>
            </li>
          
            <li>
              <a href="#NewObject" title="NewObject(name, opts)" data-gtmc="doc outline link">
                NewObject(name, opts)
              </a>
            </li>
          
            <li>
              <a href="#NewObjectHandler" title="NewObjectHandler(fn, opts)" data-gtmc="doc outline link">
                NewObjectHandler(fn, opts)
              </a>
            </li>
          
            <li>
              <a href="#NewObjectSharedHandler" title="NewObjectSharedHandler(fn, opts)" data-gtmc="doc outline link">
                NewObjectSharedHandler(fn, opts)
              </a>
            </li>
          
            <li>
              <a href="#NewService" title="NewService(name, opts)" data-gtmc="doc outline link">
                NewService(name, opts)
              </a>
            </li>
          
            <li>
              <a href="#NewServiceHandler" title="NewServiceHandler(fn, opts)" data-gtmc="doc outline link">
                NewServiceHandler(fn, opts)
              </a>
            </li>
          
            <li>
              <a href="#NewWorkflow" title="NewWorkflow(name, opts)" data-gtmc="doc outline link">
                NewWorkflow(name, opts)
              </a>
            </li>
          
            <li>
              <a href="#NewWorkflowHandler" title="NewWorkflowHandler(fn, opts)" data-gtmc="doc outline link">
                NewWorkflowHandler(fn, opts)
              </a>
            </li>
          
            <li>
              <a href="#NewWorkflowSharedHandler" title="NewWorkflowSharedHandler(fn, opts)" data-gtmc="doc outline link">
                NewWorkflowSharedHandler(fn, opts)
              </a>
            </li>
          
            <li>
              <a href="#PauseOnMaxAttempts" title="PauseOnMaxAttempts()" data-gtmc="doc outline link">
                PauseOnMaxAttempts()
              </a>
            </li>
          
            <li>
              <a href="#Rand" title="Rand(ctx)" data-gtmc="doc outline link">
                Rand(ctx)
              </a>
            </li>
          
            <li>
              <a href="#RandSource" title="RandSource(ctx)" data-gtmc="doc outline link">
                RandSource(ctx)
              </a>
            </li>
          
            <li>
              <a href="#RejectAwakeable" title="RejectAwakeable(ctx, id, reason)" data-gtmc="doc outline link">
                RejectAwakeable(ctx, id, reason)
              </a>
            </li>
          
            <li>
              <a href="#RejectSignal" title="RejectSignal(ctx, invocationID, name, reason)" data-gtmc="doc outline link">
                RejectSignal(ctx, invocationID, name, reason)
              </a>
            </li>
          
            <li>
              <a href="#ResolveAwakeable" title="ResolveAwakeable(ctx, id, value, options)" data-gtmc="doc outline link">
                ResolveAwakeable(ctx, id, value, options)
              </a>
            </li>
          
            <li>
              <a href="#ResolveSignal" title="ResolveSignal(ctx, invocationID, name, value, options)" data-gtmc="doc outline link">
                ResolveSignal(ctx, invocationID, name, value, options)
              </a>
            </li>
          
            <li>
              <a href="#Set" title="Set(ctx, key, value, options)" data-gtmc="doc outline link">
                Set(ctx, key, value, options)
              </a>
            </li>
          
            <li>
              <a href="#UUID" title="UUID(ctx)" data-gtmc="doc outline link">
                UUID(ctx)
              </a>
            </li>
          
            <li>
              <a href="#Wait" title="Wait(ctx, futs)" data-gtmc="doc outline link">
                Wait(ctx, futs)
              </a>
            </li>
          
            <li>
              <a href="#WaitFirst" title="WaitFirst(ctx, futs)" data-gtmc="doc outline link">
                WaitFirst(ctx, futs)
              </a>
            </li>
          
            <li>
              <a href="#WithAbortTimeout" title="WithAbortTimeout(abortTimeout)" data-gtmc="doc outline link">
                WithAbortTimeout(abortTimeout)
              </a>
            </li>
          
            <li>
              <a href="#WithCodec" title="WithCodec(codec)" data-gtmc="doc outline link">
                WithCodec(codec)
              </a>
            </li>
          
            <li>
              <a href="#WithDelay" title="WithDelay(delay)" data-gtmc="doc outline link">
                WithDelay(delay)
              </a>
            </li>
          
            <li>
              <a href="#WithDocumentation" title="WithDocumentation(documentation)" data-gtmc="doc outline link">
                WithDocumentation(documentation)
              </a>
            </li>
          
            <li>
              <a href="#WithEnableLazyState" title="WithEnableLazyState(enableLazyState)" data-gtmc="doc outline link">
                WithEnableLazyState(enableLazyState)
              </a>
            </li>
          
            <li>
              <a href="#WithHeaders" title="WithHeaders(headers)" data-gtmc="doc outline link">
                WithHeaders(headers)
              </a>
            </li>
          
            <li>
              <a href="#WithIdempotencyKey" title="WithIdempotencyKey(idempotencyKey)" data-gtmc="doc outline link">
                WithIdempotencyKey(idempotencyKey)
              </a>
            </li>
          
            <li>
              <a href="#WithIdempotencyRetention" title="WithIdempotencyRetention(idempotencyRetention)" data-gtmc="doc outline link">
                WithIdempotencyRetention(idempotencyRetention)
              </a>
            </li>
          
            <li>
              <a href="#WithInactivityTimeout" title="WithInactivityTimeout(inactivityTimeout)" data-gtmc="doc outline link">
                WithInactivityTimeout(inactivityTimeout)
              </a>
            </li>
          
            <li>
              <a href="#WithIngressPrivate" title="WithIngressPrivate(ingressPrivate)" data-gtmc="doc outline link">
                WithIngressPrivate(ingressPrivate)
              </a>
            </li>
          
            <li>
              <a href="#WithInitialRetryInterval" title="WithInitialRetryInterval(d)" data-gtmc="doc outline link">
                WithInitialRetryInterval(d)
              </a>
            </li>
          
            <li>
              <a href="#WithInputCodec" title="WithInputCodec(codec)" data-gtmc="doc outline link">
                WithInputCodec(codec)
              </a>
            </li>
          
            <li>
              <a href="#WithInvocationRetryPolicy" title="WithInvocationRetryPolicy(opts)" data-gtmc="doc outline link">
                WithInvocationRetryPolicy(opts)
              </a>
            </li>
          
            <li>
              <a href="#WithJournalRetention" title="WithJournalRetention(journalRetention)" data-gtmc="doc outline link">
                WithJournalRetention(journalRetention)
              </a>
            </li>
          
            <li>
              <a href="#WithLimitKey" title="WithLimitKey(limitKey)" data-gtmc="doc outline link">
                WithLimitKey(limitKey)
              </a>
            </li>
          
            <li>
              <a href="#WithMaxRetryAttempts" title="WithMaxRetryAttempts(maxAttempts)" data-gtmc="doc outline link">
                WithMaxRetryAttempts(maxAttempts)
              </a>
            </li>
          
            <li>
              <a href="#WithMaxRetryDuration" title="WithMaxRetryDuration(d)" data-gtmc="doc outline link">
                WithMaxRetryDuration(d)
              </a>
            </li>
          
            <li>
              <a href="#WithMaxRetryInterval" title="WithMaxRetryInterval(d)" data-gtmc="doc outline link">
                WithMaxRetryInterval(d)
              </a>
            </li>
          
            <li>
              <a href="#WithMetadata" title="WithMetadata(metadataKey, metadataValue)" data-gtmc="doc outline link">
                WithMetadata(metadataKey, metadataValue)
              </a>
            </li>
          
            <li>
              <a href="#WithMetadataMap" title="WithMetadataMap(metadata)" data-gtmc="doc outline link">
                WithMetadataMap(metadata)
              </a>
            </li>
          
            <li>
              <a href="#WithMockContext" title="WithMockContext(ctx)" data-gtmc="doc outline link">
                WithMockContext(ctx)
              </a>
            </li>
          
            <li>
              <a href="#WithName" title="WithName(name)" data-gtmc="doc outline link">
                WithName(name)
              </a>
            </li>
          
            <li>
              <a href="#WithOutputCodec" title="WithOutputCodec(codec)" data-gtmc="doc outline link">
                WithOutputCodec(codec)
              </a>
            </li>
          
            <li>
              <a href="#WithRetryIntervalFactor" title="WithRetryIntervalFactor(f)" data-gtmc="doc outline link">
                WithRetryIntervalFactor(f)
              </a>
            </li>
          
            <li>
              <a href="#WithScope" title="WithScope(scope)" data-gtmc="doc outline link">
                WithScope(scope)
              </a>
            </li>
          
            <li>
              <a href="#WithValue" title="WithValue(restateCtx, key, val)" data-gtmc="doc outline link">
                WithValue(restateCtx, key, val)
              </a>
            </li>
          
            <li>
              <a href="#WithWorkflowRetention" title="WithWorkflowRetention(workflowCompletionRetention)" data-gtmc="doc outline link">
                WithWorkflowRetention(workflowCompletionRetention)
              </a>
            </li>
          
            <li>
              <a href="#WrapContext" title="WrapContext(restateCtx, wrappedCtx)" data-gtmc="doc outline link">
                WrapContext(restateCtx, wrappedCtx)
              </a>
            </li>
          
        </ul>
      
    </li>
    <li class="DocNav-types">
      <a href="#pkg-types" data-gtmc="doc outline link">
        Types
      </a>
      <ul>
        
          
          <li>
            <a href="#AfterFuture" title="type AfterFuture" data-gtmc="doc outline link">
              type AfterFuture
            </a>
            
              <ul>
                
                  <li>
                    <a href="#After" title="After(ctx, d, opts)"
                        data-gtmc="doc outline link">
                      After(ctx, d, opts)
                    </a>
                  </li>
                
                
              </ul>
             
          </li>
        
          
          <li>
            <a href="#AttachFuture" title="type AttachFuture" data-gtmc="doc outline link">
              type AttachFuture
            </a>
            
              <ul>
                
                  <li>
                    <a href="#AttachInvocation" title="AttachInvocation(ctx, invocationId, options)"
                        data-gtmc="doc outline link">
                      AttachInvocation(ctx, invocationId, options)
                    </a>
                  </li>
                
                
              </ul>
             
          </li>
        
          
          <li>
            <a href="#AttachOption" title="type AttachOption" data-gtmc="doc outline link">
              type AttachOption
            </a>
             
          </li>
        
          
          <li>
            <a href="#AwakeableFuture" title="type AwakeableFuture" data-gtmc="doc outline link">
              type AwakeableFuture
            </a>
            
              <ul>
                
                  <li>
                    <a href="#Awakeable" title="Awakeable(ctx, options)"
                        data-gtmc="doc outline link">
                      Awakeable(ctx, options)
                    </a>
                  </li>
                
                
              </ul>
             
          </li>
        
          
          <li>
            <a href="#AwakeableOption" title="type AwakeableOption" data-gtmc="doc outline link">
              type AwakeableOption
            </a>
             
          </li>
        
          
          <li>
            <a href="#Client" title="type Client" data-gtmc="doc outline link">
              type Client
            </a>
            
              <ul>
                
                  <li>
                    <a href="#Object" title="Object(ctx, service, key, method, options)"
                        data-gtmc="doc outline link">
                      Object(ctx, service, key, method, options)
                    </a>
                  </li>
                
                  <li>
                    <a href="#Service" title="Service(ctx, service, method, options)"
                        data-gtmc="doc outline link">
                      Service(ctx, service, method, options)
                    </a>
                  </li>
                
                  <li>
                    <a href="#WithRequestType" title="WithRequestType(inner)"
                        data-gtmc="doc outline link">
                      WithRequestType(inner)
                    </a>
                  </li>
                
                  <li>
                    <a href="#Workflow" title="Workflow(ctx, service, workflowID, method, options)"
                        data-gtmc="doc outline link">
                      Workflow(ctx, service, workflowID, method, options)
                    </a>
                  </li>
                
                
              </ul>
             
          </li>
        
          
          <li>
            <a href="#ClientOption" title="type ClientOption" data-gtmc="doc outline link">
              type ClientOption
            </a>
             
          </li>
        
          
          <li>
            <a href="#Code" title="type Code" data-gtmc="doc outline link">
              type Code
            </a>
             
          </li>
        
          
          <li>
            <a href="#Context" title="type Context" data-gtmc="doc outline link">
              type Context
            </a>
             
          </li>
        
          
          <li>
            <a href="#DurablePromise" title="type DurablePromise" data-gtmc="doc outline link">
              type DurablePromise
            </a>
            
              <ul>
                
                  <li>
                    <a href="#Promise" title="Promise(ctx, name, options)"
                        data-gtmc="doc outline link">
                      Promise(ctx, name, options)
                    </a>
                  </li>
                
                
              </ul>
             
          </li>
        
          
          <li>
            <a href="#ErrorCodeOption" title="type ErrorCodeOption" data-gtmc="doc outline link">
              type ErrorCodeOption
            </a>
            
              <ul>
                
                  <li>
                    <a href="#WithErrorCode" title="WithErrorCode(code)"
                        data-gtmc="doc outline link">
                      WithErrorCode(code)
                    </a>
                  </li>
                
                
              </ul>
             
          </li>
        
          
          <li>
            <a href="#Future" title="type Future" data-gtmc="doc outline link">
              type Future
            </a>
             
          </li>
        
          
          <li>
            <a href="#GetOption" title="type GetOption" data-gtmc="doc outline link">
              type GetOption
            </a>
             
          </li>
        
          
          <li>
            <a href="#HandlerOption" title="type HandlerOption" data-gtmc="doc outline link">
              type HandlerOption
            </a>
             
          </li>
        
          
          <li>
            <a href="#Invocation" title="type Invocation" data-gtmc="doc outline link">
              type Invocation
            </a>
             
          </li>
        
          
          <li>
            <a href="#InvocationRetryPolicy" title="type InvocationRetryPolicy" data-gtmc="doc outline link">
              type InvocationRetryPolicy
            </a>
             
          </li>
        
          
          <li>
            <a href="#InvocationRetryPolicyOption" title="type InvocationRetryPolicyOption" data-gtmc="doc outline link">
              type InvocationRetryPolicyOption
            </a>
             
          </li>
        
          
          <li>
            <a href="#MockableContext" title="type MockableContext" data-gtmc="doc outline link">
              type MockableContext
            </a>
             
          </li>
        
          
          <li>
            <a href="#ObjectContext" title="type ObjectContext" data-gtmc="doc outline link">
              type ObjectContext
            </a>
             
          </li>
        
          
          <li>
            <a href="#ObjectHandlerFn" title="type ObjectHandlerFn" data-gtmc="doc outline link">
              type ObjectHandlerFn
            </a>
             
          </li>
        
          
          <li>
            <a href="#ObjectSharedContext" title="type ObjectSharedContext" data-gtmc="doc outline link">
              type ObjectSharedContext
            </a>
             
          </li>
        
          
          <li>
            <a href="#ObjectSharedHandlerFn" title="type ObjectSharedHandlerFn" data-gtmc="doc outline link">
              type ObjectSharedHandlerFn
            </a>
             
          </li>
        
          
          <li>
            <a href="#OnMaxAttempts" title="type OnMaxAttempts" data-gtmc="doc outline link">
              type OnMaxAttempts
            </a>
             
          </li>
        
          
          <li>
            <a href="#PromiseOption" title="type PromiseOption" data-gtmc="doc outline link">
              type PromiseOption
            </a>
             
          </li>
        
          
          <li>
            <a href="#Request" title="type Request" data-gtmc="doc outline link">
              type Request
            </a>
             
          </li>
        
          
          <li>
            <a href="#RequestOption" title="type RequestOption" data-gtmc="doc outline link">
              type RequestOption
            </a>
             
          </li>
        
          
          <li>
            <a href="#ResolveAwakeableOption" title="type ResolveAwakeableOption" data-gtmc="doc outline link">
              type ResolveAwakeableOption
            </a>
             
          </li>
        
          
          <li>
            <a href="#ResolveSignalOption" title="type ResolveSignalOption" data-gtmc="doc outline link">
              type ResolveSignalOption
            </a>
             
          </li>
        
          
          <li>
            <a href="#ResponseFuture" title="type ResponseFuture" data-gtmc="doc outline link">
              type ResponseFuture
            </a>
             
          </li>
        
          
          <li>
            <a href="#RetryableError" title="type RetryableError" data-gtmc="doc outline link">
              type RetryableError
            </a>
            
              <ul>
                
                  <li>
                    <a href="#AsRetryableError" title="AsRetryableError(err)"
                        data-gtmc="doc outline link">
                      AsRetryableError(err)
                    </a>
                  </li>
                
                  <li>
                    <a href="#RetryableErrorf" title="RetryableErrorf(format, a)"
                        data-gtmc="doc outline link">
                      RetryableErrorf(format, a)
                    </a>
                  </li>
                
                  <li>
                    <a href="#ToRetryableError" title="ToRetryableError(err, opts)"
                        data-gtmc="doc outline link">
                      ToRetryableError(err, opts)
                    </a>
                  </li>
                
                
              </ul>
             
          </li>
        
          
          <li>
            <a href="#RetryableErrorOption" title="type RetryableErrorOption" data-gtmc="doc outline link">
              type RetryableErrorOption
            </a>
             
          </li>
        
          
          <li>
            <a href="#RunAsyncFuture" title="type RunAsyncFuture" data-gtmc="doc outline link">
              type RunAsyncFuture
            </a>
            
              <ul>
                
                  <li>
                    <a href="#RunAsync" title="RunAsync(ctx, fn, options)"
                        data-gtmc="doc outline link">
                      RunAsync(ctx, fn, options)
                    </a>
                  </li>
                
                
              </ul>
             
          </li>
        
          
          <li>
            <a href="#RunContext" title="type RunContext" data-gtmc="doc outline link">
              type RunContext
            </a>
             
          </li>
        
          
          <li>
            <a href="#RunOption" title="type RunOption" data-gtmc="doc outline link">
              type RunOption
            </a>
             
          </li>
        
          
          <li>
            <a href="#SendClient" title="type SendClient" data-gtmc="doc outline link">
              type SendClient
            </a>
            
              <ul>
                
                  <li>
                    <a href="#ObjectSend" title="ObjectSend(ctx, service, key, method, options)"
                        data-gtmc="doc outline link">
                      ObjectSend(ctx, service, key, method, options)
                    </a>
                  </li>
                
                  <li>
                    <a href="#ServiceSend" title="ServiceSend(ctx, service, method, options)"
                        data-gtmc="doc outline link">
                      ServiceSend(ctx, service, method, options)
                    </a>
                  </li>
                
                  <li>
                    <a href="#WorkflowSend" title="WorkflowSend(ctx, service, workflowID, method, options)"
                        data-gtmc="doc outline link">
                      WorkflowSend(ctx, service, workflowID, method, options)
                    </a>
                  </li>
                
                
              </ul>
             
          </li>
        
          
          <li>
            <a href="#SendOption" title="type SendOption" data-gtmc="doc outline link">
              type SendOption
            </a>
             
          </li>
        
          
          <li>
            <a href="#ServiceDefinition" title="type ServiceDefinition" data-gtmc="doc outline link">
              type ServiceDefinition
            </a>
            
              <ul>
                
                  <li>
                    <a href="#Reflect" title="Reflect(rcvr, opts)"
                        data-gtmc="doc outline link">
                      Reflect(rcvr, opts)
                    </a>
                  </li>
                
                
              </ul>
             
          </li>
        
          
          <li>
            <a href="#ServiceDefinitionOption" title="type ServiceDefinitionOption" data-gtmc="doc outline link">
              type ServiceDefinitionOption
            </a>
             
          </li>
        
          
          <li>
            <a href="#ServiceHandlerFn" title="type ServiceHandlerFn" data-gtmc="doc outline link">
              type ServiceHandlerFn
            </a>
             
          </li>
        
          
          <li>
            <a href="#SetOption" title="type SetOption" data-gtmc="doc outline link">
              type SetOption
            </a>
             
          </li>
        
          
          <li>
            <a href="#SignalFuture" title="type SignalFuture" data-gtmc="doc outline link">
              type SignalFuture
            </a>
            
              <ul>
                
                  <li>
                    <a href="#Signal" title="Signal(ctx, name, options)"
                        data-gtmc="doc outline link">
                      Signal(ctx, name, options)
                    </a>
                  </li>
                
                
              </ul>
             
          </li>
        
          
          <li>
            <a href="#SignalOption" title="type SignalOption" data-gtmc="doc outline link">
              type SignalOption
            </a>
             
          </li>
        
          
          <li>
            <a href="#SleepOption" title="type SleepOption" data-gtmc="doc outline link">
              type SleepOption
            </a>
             
          </li>
        
          
          <li>
            <a href="#StringMap" title="type StringMap" data-gtmc="doc outline link">
              type StringMap
            </a>
             
          </li>
        
          
          <li>
            <a href="#TerminalError" title="type TerminalError" data-gtmc="doc outline link">
              type TerminalError
            </a>
            
              <ul>
                
                  <li>
                    <a href="#AsTerminalError" title="AsTerminalError(err)"
                        data-gtmc="doc outline link">
                      AsTerminalError(err)
                    </a>
                  </li>
                
                  <li>
                    <a href="#Get" title="Get(ctx, key, options)"
                        data-gtmc="doc outline link">
                      Get(ctx, key, options)
                    </a>
                  </li>
                
                  <li>
                    <a href="#Keys" title="Keys(ctx)"
                        data-gtmc="doc outline link">
                      Keys(ctx)
                    </a>
                  </li>
                
                  <li>
                    <a href="#Run" title="Run(ctx, fn, options)"
                        data-gtmc="doc outline link">
                      Run(ctx, fn, options)
                    </a>
                  </li>
                
                  <li>
                    <a href="#RunVoid" title="RunVoid(ctx, fn, options)"
                        data-gtmc="doc outline link">
                      RunVoid(ctx, fn, options)
                    </a>
                  </li>
                
                  <li>
                    <a href="#Sleep" title="Sleep(ctx, d, opts)"
                        data-gtmc="doc outline link">
                      Sleep(ctx, d, opts)
                    </a>
                  </li>
                
                  <li>
                    <a href="#TerminalErrorf" title="TerminalErrorf(format, a)"
                        data-gtmc="doc outline link">
                      TerminalErrorf(format, a)
                    </a>
                  </li>
                
                  <li>
                    <a href="#ToTerminalError" title="ToTerminalError(err, opts)"
                        data-gtmc="doc outline link">
                      ToTerminalError(err, opts)
                    </a>
                  </li>
                
                
              </ul>
             
          </li>
        
          
          <li>
            <a href="#TerminalErrorOption" title="type TerminalErrorOption" data-gtmc="doc outline link">
              type TerminalErrorOption
            </a>
             
          </li>
        
          
          <li>
            <a href="#Void" title="type Void" data-gtmc="doc outline link">
              type Void
            </a>
             
          </li>
        
          
          <li>
            <a href="#WaitIterator" title="type WaitIterator" data-gtmc="doc outline link">
              type WaitIterator
            </a>
            
              <ul>
                
                  <li>
                    <a href="#WaitIter" title="WaitIter(ctx, futs)"
                        data-gtmc="doc outline link">
                      WaitIter(ctx, futs)
                    </a>
                  </li>
                
                
              </ul>
             
          </li>
        
          
          <li>
            <a href="#WorkflowContext" title="type WorkflowContext" data-gtmc="doc outline link">
              type WorkflowContext
            </a>
             
          </li>
        
          
          <li>
            <a href="#WorkflowHandlerFn" title="type WorkflowHandlerFn" data-gtmc="doc outline link">
              type WorkflowHandlerFn
            </a>
             
          </li>
        
          
          <li>
            <a href="#WorkflowSharedContext" title="type WorkflowSharedContext" data-gtmc="doc outline link">
              type WorkflowSharedContext
            </a>
             
          </li>
        
          
          <li>
            <a href="#WorkflowSharedHandlerFn" title="type WorkflowSharedHandlerFn" data-gtmc="doc outline link">
              type WorkflowSharedHandlerFn
            </a>
             
          </li>
         
      </ul>
    </li>
  
  
</ul>

      </li>
    
    
      <li>
        <a href="#section-sourcefiles" data-gtmc="outline link">
          Source Files
        </a>
      </li>
    
    
      <li>
        <a href="#section-directories" data-gtmc="outline link">
          Directories
        </a>
      </li>
    
  </ul>

  </div>
  <div class="go-Main-navMobile js-mainNavMobile">
    <label class="go-Label">
      <select class="go-Select">
        
          <option selected disabled>README</option>
        
      </select>
    </label>
  </div>
</nav>
    <article class="go-Main-article js-mainContent">
  <div class="UnitDetails" data-test-id="UnitDetails" style="display: block;">
    <div class="UnitDetails-content js-unitDetailsContent" data-test-id="UnitDetails-content">
      
        
  <div class="UnitReadme UnitReadme--expanded js-readme">
    <h2 class="UnitReadme-title" id="section-readme">
      <img class="go-Icon" height="24" width="24" src="/static/shared/icon/chrome_reader_mode_gm_grey_24dp.svg" alt="">
      README
      <a class="UnitReadme-idLink" href="#section-readme" title="Go to Readme" aria-label="Go to Readme">¶</a>
    </h2>
    
      <div class="UnitReadme-content" data-test-id="Unit-readmeContent">
        <div class="Overview-readmeContent js-readmeContent"><p><a href="https://pkg.go.dev/github.com/restatedev/sdk-go" rel="nofollow"><img src="https://pkg.go.dev/badge/github.com/restatedev/sdk-go.svg" alt="Go Reference"/></a>
<a href="https://github.com/restatedev/sdk-go/actions/workflows/test.yaml" rel="nofollow"><img src="https://github.com/restatedev/sdk-go/actions/workflows/test.yaml/badge.svg" alt="Go"/></a></p>
<h3 class="h1" id="readme-restate-go-sdk">Restate Go SDK</h3>
<p><a href="https://restate.dev/" rel="nofollow">Restate</a> is a system for easily building resilient applications using <em>distributed durable async/await</em>. This repository contains the Restate SDK for writing services in <strong>Golang</strong>.</p>
<h4 class="h2" id="readme-community">Community</h4>
<ul>
<li>🤗️ <a href="https://discord.gg/skW3AZ6uGd" rel="nofollow">Join our online community</a> for help, sharing feedback and talking to the community.</li>
<li>📖 <a href="https://docs.restate.dev" rel="nofollow">Check out our documentation</a> to get quickly started!</li>
<li>📣 <a href="https://twitter.com/restatedev" rel="nofollow">Follow us on Twitter</a> for staying up to date.</li>
<li>🙋 <a href="https://github.com/restatedev/sdk-java/issues" rel="nofollow">Create a GitHub issue</a> for requesting a new feature or reporting a problem.</li>
<li>🏠 <a href="https://github.com/restatedev" rel="nofollow">Visit our GitHub org</a> for exploring other repositories.</li>
</ul>
<h4 class="h2" id="readme-prerequisites">Prerequisites</h4>
<ul>
<li>Go: &gt;= 1.24.0</li>
</ul>
<h4 class="h2" id="readme-examples">Examples</h4>
<p>This repo contains an <a href="https://github.com/restatedev/sdk-go/blob/v1.1.0/examples" rel="nofollow">example</a> based on the <a href="https://github.com/restatedev/examples/tree/main/tutorials/tour-of-restate-go" rel="nofollow">Ticket Reservation Service</a>.</p>
<p>You can also check a list of examples available here: <a href="https://github.com/restatedev/examples?tab=readme-ov-file#go" rel="nofollow">https://github.com/restatedev/examples?tab=readme-ov-file#go</a></p>
<h5 class="h3" id="readme-how-to-use-the-example">How to use the example</h5>
<p>Download and run restate, as described here <a href="https://github.com/restatedev/restate/releases/" rel="nofollow">v1.x</a></p>
<pre><code>restate-server
</code></pre>
<p>In another terminal run the example</p>
<pre><code>cd restate-sdk-go/example
go run .
</code></pre>
<p>In a third terminal register:</p>
<pre><code>restate deployments register http://localhost:9080
</code></pre>
<p>And do the following steps</p>
<ul>
<li>Add tickets to basket</li>
</ul>
<pre><code>curl -v localhost:8080/UserSession/azmy/AddTicket \
    -H &#39;content-type: application/json&#39; \
    -d &#39;&#34;ticket-1&#34;&#39;

# true
curl -v localhost:8080/UserSession/azmy/AddTicket \
    -H &#39;content-type: application/json&#39; \
    -d &#39;&#34;ticket-2&#34;&#39;
# true
</code></pre>
<p>Trying adding the same tickets again should return <code>false</code> since they are already reserved. If you didn&#39;t check out the tickets in 15min (if you are impatient change the delay in code to make it shorter)</p>
<ul>
<li>Check out</li>
</ul>
<pre><code>curl localhost:8080/UserSession/azmy/Checkout
# true
</code></pre>
<h4 class="h2" id="readme-ingress-sdk">Ingress SDK</h4>
<p>When you need to call restate handlers or attach to invocations from outside the restate context,
use the <a href="https://github.com/restatedev/sdk-go/blob/v1.1.0/examples/client/main.go" rel="nofollow">ingress SDK</a>.</p>
<h4 class="h2" id="readme-versions">Versions</h4>
<p>This library follows <a href="https://semver.org/" rel="nofollow">Semantic Versioning</a>.</p>
<p><strong>Upgrading from 0.x?</strong> See the <a href="https://github.com/restatedev/sdk-go/blob/v1.1.0/MIGRATION.md" rel="nofollow">migration guide</a>.</p>
<p>Compatibility with Restate Server:</p>
<table>
<thead>
<tr>
<th>Restate Server</th>
<th>sdk-go 1.0</th>
</tr>
</thead>
<tbody>
<tr>
<td>&lt; 1.3</td>
<td>❌</td>
</tr>
<tr>
<td>1.3</td>
<td>✅ <sup>(1)(2)</sup></td>
</tr>
<tr>
<td>1.4</td>
<td>✅ <sup>(2)</sup></td>
</tr>
<tr>
<td>1.5</td>
<td>✅</td>
</tr>
<tr>
<td>1.6</td>
<td>✅</td>
</tr>
<tr>
<td>1.7</td>
<td>✅</td>
</tr>
</tbody>
</table>
<p><sup>(1)</sup> <code>WithAbortTimeout</code>, <code>WithEnableLazyState</code>, <code>WithIdempotencyRetention</code>, <code>WithInactivityTimeout</code>, <code>WithIngressPrivate</code>, <code>WithJournalRetention</code> and <code>WithWorkflowRetention</code> require Restate Server &gt;= 1.4. Check the in-code documentation for more details.</p>
<p><sup>(2)</sup> <code>WithInvocationRetryPolicy</code> requires Restate Server &gt;= 1.5. Check the in-code documentation for more details.</p>
<p>Older <code>0.x</code> SDK releases are legacy and deprecated; see the <a href="https://github.com/restatedev/restate/releases/tag/v1.5.0" rel="nofollow">Restate 1.5 release notes</a> for their compatibility and deprecation details.</p>
<h4 class="h2" id="readme-contributing">Contributing</h4>
<p>We’re excited if you join the Restate community and start contributing!
Whether it is feature requests, bug reports, ideas &amp; feedback or PRs, we appreciate any and all contributions.
We know that your time is precious and, therefore, deeply value any effort to contribute!</p>
</div>
      </div>
      <button class="UnitReadme-expandLink js-readmeExpand"
          data-test-id="readme-expand" data-gtmc="readme button"
          aria-label="Expand Readme">Expand ▾</button>
      <button class="UnitReadme-collapseLink js-readmeCollapse"
          data-test-id="readme-collapse" data-gtmc="readme button"
          aria-label="Expand Readme">Collapse ▴</button>
    
  </div>

      
      
        
          
  <div class="UnitDoc">
    <h2 class="UnitDoc-title" id="section-documentation">
      <img class="go-Icon" height="24" width="24" src="/static/shared/icon/code_gm_grey_24dp.svg" alt="">
      Documentation
      <a class="UnitDoc-idLink" href="#section-documentation" title="Go to Documentation" aria-label="Go to Documentation">¶</a>
    </h2>
    
  
    
  

    <div class="Documentation js-documentation">
      
        

<div class="Documentation-content js-docContent"> <section class="Documentation-index">
    <h3 id="pkg-index" class="Documentation-indexHeader">Index <a href="#pkg-index" title="Go to Index" aria-label="Go to Index">¶</a></h3>

<ul class="Documentation-indexList">
<li class="Documentation-indexVariables"><a href="#pkg-variables">Variables</a></li>
<li class="Documentation-indexFunction">
        <a href="#CancelInvocation">func CancelInvocation(ctx Context, invocationId string)</a></li>
<li class="Documentation-indexFunction">
        <a href="#Clear">func Clear(ctx ObjectContext, key string)</a></li>
<li class="Documentation-indexFunction">
        <a href="#ClearAll">func ClearAll(ctx ObjectContext)</a></li>
<li class="Documentation-indexFunction">
        <a href="#IsRetryableError">func IsRetryableError(err error) bool</a></li>
<li class="Documentation-indexFunction">
        <a href="#IsTerminalError">func IsTerminalError(err error) bool</a></li>
<li class="Documentation-indexFunction">
        <a href="#Key">func Key(ctx ObjectSharedContext) string</a></li>
<li class="Documentation-indexFunction">
        <a href="#KillOnMaxAttempts">func KillOnMaxAttempts() withOnMaxAttempts</a></li>
<li class="Documentation-indexFunction">
        <a href="#NewObject">func NewObject(name string, opts ...options.ServiceDefinitionOption) *object</a></li>
<li class="Documentation-indexFunction">
        <a href="#NewObjectHandler">func NewObjectHandler[I any, O any](fn ObjectHandlerFn[I, O], opts ...options.HandlerOption) *objectHandler[I, O]</a></li>
<li class="Documentation-indexFunction">
        <a href="#NewObjectSharedHandler">func NewObjectSharedHandler[I any, O any](fn ObjectSharedHandlerFn[I, O], opts ...options.HandlerOption) *objectHandler[I, O]</a></li>
<li class="Documentation-indexFunction">
        <a href="#NewService">func NewService(name string, opts ...options.ServiceDefinitionOption) *service</a></li>
<li class="Documentation-indexFunction">
        <a href="#NewServiceHandler">func NewServiceHandler[I any, O any](fn ServiceHandlerFn[I, O], opts ...options.HandlerOption) *serviceHandler[I, O]</a></li>
<li class="Documentation-indexFunction">
        <a href="#NewWorkflow">func NewWorkflow(name string, opts ...options.ServiceDefinitionOption) *workflow</a></li>
<li class="Documentation-indexFunction">
        <a href="#NewWorkflowHandler">func NewWorkflowHandler[I any, O any](fn WorkflowHandlerFn[I, O], opts ...options.HandlerOption) *workflowHandler[I, O]</a></li>
<li class="Documentation-indexFunction">
        <a href="#NewWorkflowSharedHandler">func NewWorkflowSharedHandler[I any, O any](fn WorkflowSharedHandlerFn[I, O], opts ...options.HandlerOption) *workflowHandler[I, O]</a></li>
<li class="Documentation-indexFunction">
        <a href="#PauseOnMaxAttempts">func PauseOnMaxAttempts() withOnMaxAttempts</a></li>
<li class="Documentation-indexFunction">
        <a href="#Rand">func Rand(ctx Context) *rand2.Rand</a></li>
<li class="Documentation-indexFunction">
        <a href="#RandSource">func RandSource(ctx Context) rand2.Source</a></li>
<li class="Documentation-indexFunction">
        <a href="#RejectAwakeable">func RejectAwakeable(ctx Context, id string, reason error)</a></li>
<li class="Documentation-indexFunction">
        <a href="#RejectSignal">func RejectSignal(ctx Context, invocationID string, name string, reason error)</a></li>
<li class="Documentation-indexFunction">
        <a href="#ResolveAwakeable">func ResolveAwakeable[T any](ctx Context, id string, value T, options ...options.ResolveAwakeableOption)</a></li>
<li class="Documentation-indexFunction">
        <a href="#ResolveSignal">func ResolveSignal[T any](ctx Context, invocationID string, name string, value T, ...)</a></li>
<li class="Documentation-indexFunction">
        <a href="#Set">func Set[T any](ctx ObjectContext, key string, value T, options ...options.SetOption)</a></li>
<li class="Documentation-indexFunction">
        <a href="#UUID">func UUID(ctx Context) uuid.UUID</a></li>
<li class="Documentation-indexFunction">
        <a href="#Wait">func Wait(ctx Context, futs ...Future) iter.Seq2[Future, TerminalError]</a></li>
<li class="Documentation-indexFunction">
        <a href="#WaitFirst">func WaitFirst(ctx Context, futs ...Future) (resultFut Future, cancellationError TerminalError)</a></li>
<li class="Documentation-indexFunction">
        <a href="#WithAbortTimeout">func WithAbortTimeout(abortTimeout time.Duration) withAbortTimeout</a></li>
<li class="Documentation-indexFunction">
        <a href="#WithCodec">func WithCodec(codec encoding.Codec) withCodec</a></li>
<li class="Documentation-indexFunction">
        <a href="#WithDelay">func WithDelay(delay time.Duration) withDelay</a></li>
<li class="Documentation-indexFunction">
        <a href="#WithDocumentation">func WithDocumentation(documentation string) withDocumentation</a></li>
<li class="Documentation-indexFunction">
        <a href="#WithEnableLazyState">func WithEnableLazyState(enableLazyState bool) withEnableLazyState</a></li>
<li class="Documentation-indexFunction">
        <a href="#WithHeaders">func WithHeaders(headers map[string]string) withHeaders</a></li>
<li class="Documentation-indexFunction">
        <a href="#WithIdempotencyKey">func WithIdempotencyKey(idempotencyKey string) withIdempotencyKey</a></li>
<li class="Documentation-indexFunction">
        <a href="#WithIdempotencyRetention">func WithIdempotencyRetention(idempotencyRetention time.Duration) withIdempotencyRetention</a></li>
<li class="Documentation-indexFunction">
        <a href="#WithInactivityTimeout">func WithInactivityTimeout(inactivityTimeout time.Duration) withInactivityTimeout</a></li>
<li class="Documentation-indexFunction">
        <a href="#WithIngressPrivate">func WithIngressPrivate(ingressPrivate bool) withIngressPrivate</a></li>
<li class="Documentation-indexFunction">
        <a href="#WithInitialRetryInterval">func WithInitialRetryInterval(d time.Duration) withInitialRetryInterval</a></li>
<li class="Documentation-indexFunction">
        <a href="#WithInputCodec">func WithInputCodec(codec encoding.Codec) withInputCodec</a></li>
<li class="Documentation-indexFunction">
        <a href="#WithInvocationRetryPolicy">func WithInvocationRetryPolicy(opts ...InvocationRetryPolicyOption) withInvocationRetryPolicy</a></li>
<li class="Documentation-indexFunction">
        <a href="#WithJournalRetention">func WithJournalRetention(journalRetention time.Duration) withJournalRetention</a></li>
<li class="Documentation-indexFunction">
        <a href="#WithLimitKey">func WithLimitKey(limitKey string) withLimitKey</a></li>
<li class="Documentation-indexFunction">
        <a href="#WithMaxRetryAttempts">func WithMaxRetryAttempts(maxAttempts uint) withMaxRetryAttempts</a></li>
<li class="Documentation-indexFunction">
        <a href="#WithMaxRetryDuration">func WithMaxRetryDuration(d time.Duration) withMaxRetryDuration</a></li>
<li class="Documentation-indexFunction">
        <a href="#WithMaxRetryInterval">func WithMaxRetryInterval(d time.Duration) withMaxRetryInterval</a></li>
<li class="Documentation-indexFunction">
        <a href="#WithMetadata">func WithMetadata(metadataKey string, metadataValue string) errors.MetadataOption</a></li>
<li class="Documentation-indexFunction">
        <a href="#WithMetadataMap">func WithMetadataMap(metadata map[string]string) errors.MetadataOption</a></li>
<li class="Documentation-indexFunction">
        <a href="#WithMockContext">func WithMockContext(ctx MockableContext) ctxWrapper</a></li>
<li class="Documentation-indexFunction">
        <a href="#WithName">func WithName(name string) withName</a></li>
<li class="Documentation-indexFunction">
        <a href="#WithOutputCodec">func WithOutputCodec(codec encoding.Codec) withOutputCodec</a></li>
<li class="Documentation-indexFunction">
        <a href="#WithRetryIntervalFactor">func WithRetryIntervalFactor(f float32) withRetryIntervalFactor</a></li>
<li class="Documentation-indexFunction">
        <a href="#WithScope">func WithScope(scope string) withScope</a></li>
<li class="Documentation-indexFunction">
        <a href="#WithValue">func WithValue[T Context](restateCtx T, key, val any) T</a></li>
<li class="Documentation-indexFunction">
        <a href="#WithWorkflowRetention">func WithWorkflowRetention(workflowCompletionRetention time.Duration) withWorkflowRetention</a></li>
<li class="Documentation-indexFunction">
        <a href="#WrapContext">func WrapContext[T Context](restateCtx T, wrappedCtx context.Context) T</a></li>
<li class="Documentation-indexType">
          <a href="#AfterFuture">type AfterFuture</a></li>
<li><ul class="Documentation-indexTypeFunctions">
<li>
            <a href="#After">func After(ctx Context, d time.Duration, opts ...options.SleepOption) AfterFuture</a></li>
</ul></li>
<li class="Documentation-indexType">
          <a href="#AttachFuture">type AttachFuture</a></li>
<li><ul class="Documentation-indexTypeFunctions">
<li>
            <a href="#AttachInvocation">func AttachInvocation[T any](ctx Context, invocationId string, options ...options.AttachOption) AttachFuture[T]</a></li>
</ul></li>
<li class="Documentation-indexType">
          <a href="#AttachOption">type AttachOption</a></li>
<li class="Documentation-indexType">
          <a href="#AwakeableFuture">type AwakeableFuture</a></li>
<li><ul class="Documentation-indexTypeFunctions">
<li>
            <a href="#Awakeable">func Awakeable[T any](ctx Context, options ...options.AwakeableOption) AwakeableFuture[T]</a></li>
</ul></li>
<li class="Documentation-indexType">
          <a href="#AwakeableOption">type AwakeableOption</a></li>
<li class="Documentation-indexType">
          <a href="#Client">type Client</a></li>
<li><ul class="Documentation-indexTypeFunctions">
<li>
            <a href="#Object">func Object[O any](ctx Context, service string, key string, method string, ...) Client[any, O]</a></li>
<li>
            <a href="#Service">func Service[O any](ctx Context, service string, method string, options ...options.ClientOption) Client[any, O]</a></li>
<li>
            <a href="#WithRequestType">func WithRequestType[I any, O any](inner Client[any, O]) Client[I, O]</a></li>
<li>
            <a href="#Workflow">func Workflow[O any](ctx Context, service string, workflowID string, method string, ...) Client[any, O]</a></li>
</ul></li>
<li class="Documentation-indexType">
          <a href="#ClientOption">type ClientOption</a></li>
<li class="Documentation-indexType">
          <a href="#Code">type Code</a></li>
<li class="Documentation-indexType">
          <a href="#Context">type Context</a></li>
<li class="Documentation-indexType">
          <a href="#DurablePromise">type DurablePromise</a></li>
<li><ul class="Documentation-indexTypeFunctions">
<li>
            <a href="#Promise">func Promise[T any](ctx WorkflowSharedContext, name string, options ...options.PromiseOption) DurablePromise[T]</a></li>
</ul></li>
<li class="Documentation-indexType">
          <a href="#ErrorCodeOption">type ErrorCodeOption</a></li>
<li><ul class="Documentation-indexTypeFunctions">
<li>
            <a href="#WithErrorCode">func WithErrorCode(code Code) ErrorCodeOption</a></li>
</ul></li>
<li class="Documentation-indexType">
          <a href="#Future">type Future</a></li>
<li class="Documentation-indexType">
          <a href="#GetOption">type GetOption</a></li>
<li class="Documentation-indexType">
          <a href="#HandlerOption">type HandlerOption</a></li>
<li class="Documentation-indexType">
          <a href="#Invocation">type Invocation</a></li>
<li class="Documentation-indexType">
          <a href="#InvocationRetryPolicy">type InvocationRetryPolicy</a></li>
<li class="Documentation-indexType">
          <a href="#InvocationRetryPolicyOption">type InvocationRetryPolicyOption</a></li>
<li class="Documentation-indexType">
          <a href="#MockableContext">type MockableContext</a></li>
<li class="Documentation-indexType">
          <a href="#ObjectContext">type ObjectContext</a></li>
<li class="Documentation-indexType">
          <a href="#ObjectHandlerFn">type ObjectHandlerFn</a></li>
<li class="Documentation-indexType">
          <a href="#ObjectSharedContext">type ObjectSharedContext</a></li>
<li class="Documentation-indexType">
          <a href="#ObjectSharedHandlerFn">type ObjectSharedHandlerFn</a></li>
<li class="Documentation-indexType">
          <a href="#OnMaxAttempts">type OnMaxAttempts</a></li>
<li class="Documentation-indexType">
          <a href="#PromiseOption">type PromiseOption</a></li>
<li class="Documentation-indexType">
          <a href="#Request">type Request</a></li>
<li class="Documentation-indexType">
          <a href="#RequestOption">type RequestOption</a></li>
<li class="Documentation-indexType">
          <a href="#ResolveAwakeableOption">type ResolveAwakeableOption</a></li>
<li class="Documentation-indexType">
          <a href="#ResolveSignalOption">type ResolveSignalOption</a></li>
<li class="Documentation-indexType">
          <a href="#ResponseFuture">type ResponseFuture</a></li>
<li class="Documentation-indexType">
          <a href="#RetryableError">type RetryableError</a></li>
<li><ul class="Documentation-indexTypeFunctions">
<li>
            <a href="#AsRetryableError">func AsRetryableError(err error) RetryableError</a></li>
<li>
            <a href="#RetryableErrorf">func RetryableErrorf(format string, a ...any) RetryableError</a></li>
<li>
            <a href="#ToRetryableError">func ToRetryableError(err error, opts ...RetryableErrorOption) RetryableError</a></li>
</ul></li>
<li class="Documentation-indexType">
          <a href="#RetryableErrorOption">type RetryableErrorOption</a></li>
<li class="Documentation-indexType">
          <a href="#RunAsyncFuture">type RunAsyncFuture</a></li>
<li><ul class="Documentation-indexTypeFunctions">
<li>
            <a href="#RunAsync">func RunAsync[T any](ctx Context, fn func(ctx RunContext) (T, error), options ...options.RunOption) RunAsyncFuture[T]</a></li>
</ul></li>
<li class="Documentation-indexType">
          <a href="#RunContext">type RunContext</a></li>
<li class="Documentation-indexType">
          <a href="#RunOption">type RunOption</a></li>
<li class="Documentation-indexType">
          <a href="#SendClient">type SendClient</a></li>
<li><ul class="Documentation-indexTypeFunctions">
<li>
            <a href="#ObjectSend">func ObjectSend(ctx Context, service string, key string, method string, ...) SendClient[any]</a></li>
<li>
            <a href="#ServiceSend">func ServiceSend(ctx Context, service string, method string, options ...options.ClientOption) SendClient[any]</a></li>
<li>
            <a href="#WorkflowSend">func WorkflowSend(ctx Context, service string, workflowID string, method string, ...) SendClient[any]</a></li>
</ul></li>
<li class="Documentation-indexType">
          <a href="#SendOption">type SendOption</a></li>
<li class="Documentation-indexType">
          <a href="#ServiceDefinition">type ServiceDefinition</a></li>
<li><ul class="Documentation-indexTypeFunctions">
<li>
            <a href="#Reflect">func Reflect(rcvr any, opts ...options.ServiceDefinitionOption) ServiceDefinition</a></li>
</ul></li>
<li class="Documentation-indexType">
          <a href="#ServiceDefinitionOption">type ServiceDefinitionOption</a></li>
<li class="Documentation-indexType">
          <a href="#ServiceHandlerFn">type ServiceHandlerFn</a></li>
<li class="Documentation-indexType">
          <a href="#SetOption">type SetOption</a></li>
<li class="Documentation-indexType">
          <a href="#SignalFuture">type SignalFuture</a></li>
<li><ul class="Documentation-indexTypeFunctions">
<li>
            <a href="#Signal">func Signal[T any](ctx Context, name string, options ...options.SignalOption) SignalFuture[T]</a></li>
</ul></li>
<li class="Documentation-indexType">
          <a href="#SignalOption">type SignalOption</a></li>
<li class="Documentation-indexType">
          <a href="#SleepOption">type SleepOption</a></li>
<li class="Documentation-indexType">
          <a href="#StringMap">type StringMap</a></li>
<li class="Documentation-indexType">
          <a href="#TerminalError">type TerminalError</a></li>
<li><ul class="Documentation-indexTypeFunctions">
<li>
            <a href="#AsTerminalError">func AsTerminalError(err error) TerminalError</a></li>
<li>
            <a href="#Get">func Get[T any](ctx ObjectSharedContext, key string, options ...options.GetOption) (output T, err TerminalError)</a></li>
<li>
            <a href="#Keys">func Keys(ctx ObjectSharedContext) ([]string, TerminalError)</a></li>
<li>
            <a href="#Run">func Run[T any](ctx Context, fn func(ctx RunContext) (T, error), options ...options.RunOption) (output T, err TerminalError)</a></li>
<li>
            <a href="#RunVoid">func RunVoid(ctx Context, fn func(ctx RunContext) error, options ...options.RunOption) TerminalError</a></li>
<li>
            <a href="#Sleep">func Sleep(ctx Context, d time.Duration, opts ...options.SleepOption) TerminalError</a></li>
<li>
            <a href="#TerminalErrorf">func TerminalErrorf(format string, a ...any) TerminalError</a></li>
<li>
            <a href="#ToTerminalError">func ToTerminalError(err error, opts ...TerminalErrorOption) TerminalError</a></li>
</ul></li>
<li class="Documentation-indexType">
          <a href="#TerminalErrorOption">type TerminalErrorOption</a></li>
<li class="Documentation-indexType">
          <a href="#Void">type Void</a></li>
<li class="Documentation-indexType">
          <a href="#WaitIterator">type WaitIterator</a></li>
<li><ul class="Documentation-indexTypeFunctions">
<li>
            <a href="#WaitIter">func WaitIter(ctx Context, futs ...Future) WaitIterator</a></li>
</ul></li>
<li class="Documentation-indexType">
          <a href="#WorkflowContext">type WorkflowContext</a></li>
<li class="Documentation-indexType">
          <a href="#WorkflowHandlerFn">type WorkflowHandlerFn</a></li>
<li class="Documentation-indexType">
          <a href="#WorkflowSharedContext">type WorkflowSharedContext</a></li>
<li class="Documentation-indexType">
          <a href="#WorkflowSharedHandlerFn">type WorkflowSharedHandlerFn</a></li>
</ul>
</section><h3 tabindex="-1" id="pkg-constants" class="Documentation-constantsHeader">Constants <a href="#pkg-constants" title="Go to Constants" aria-label="Go to Constants">¶</a></h3>

  <section class="Documentation-constants"><p class="Documentation-empty">This section is empty.</p></section>

  <h3 tabindex="-1" id="pkg-variables" class="Documentation-variablesHeader">Variables <a href="#pkg-variables" title="Go to Variables" aria-label="Go to Variables">¶</a></h3>

  <section class="Documentation-variables">
    <div class="Documentation-declaration">
      <span class="Documentation-declarationLink"><a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/codec.go#L132">View Source</a></span>
      <pre><span id="WithBinary" data-kind="variable">var WithBinary = <a href="#WithCodec">WithCodec</a>(<a href="/github.com/restatedev/sdk-go@v1.1.0/encoding">encoding</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/encoding#BinaryCodec">BinaryCodec</a>)</span></pre>
    </div>
  <p>WithBinary is an option to specify the use of <a href="/github.com/restatedev/sdk-go@v1.1.0/encoding#BinaryCodec">encoding.BinaryCodec</a> for (de)serialisation
</p>

    <div class="Documentation-declaration">
      <span class="Documentation-declarationLink"><a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/codec.go#L135">View Source</a></span>
      <pre><span id="WithJSON" data-kind="variable">var WithJSON = <a href="#WithCodec">WithCodec</a>(<a href="/github.com/restatedev/sdk-go@v1.1.0/encoding">encoding</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/encoding#JSONCodec">JSONCodec</a>)</span></pre>
    </div>
  <p>WithJSON is an option to specify the use of <a href="/github.com/restatedev/sdk-go@v1.1.0/encoding#JSONCodec">encoding.JSONCodec</a> for (de)serialisation
</p>

    <div class="Documentation-declaration">
      <span class="Documentation-declarationLink"><a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/codec.go#L126">View Source</a></span>
      <pre><span id="WithProto" data-kind="variable">var WithProto = <a href="#WithCodec">WithCodec</a>(<a href="/github.com/restatedev/sdk-go@v1.1.0/encoding">encoding</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/encoding#ProtoCodec">ProtoCodec</a>)</span></pre>
    </div>
  <p>WithProto is an option to specify the use of <a href="/github.com/restatedev/sdk-go@v1.1.0/encoding#ProtoCodec">encoding.ProtoCodec</a> for (de)serialisation
</p>

    <div class="Documentation-declaration">
      <span class="Documentation-declarationLink"><a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/codec.go#L129">View Source</a></span>
      <pre><span id="WithProtoJSON" data-kind="variable">var WithProtoJSON = <a href="#WithCodec">WithCodec</a>(<a href="/github.com/restatedev/sdk-go@v1.1.0/encoding">encoding</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/encoding#ProtoJSONCodec">ProtoJSONCodec</a>)</span></pre>
    </div>
  <p>WithProtoJSON is an option to specify the use of <a href="/github.com/restatedev/sdk-go@v1.1.0/encoding#ProtoJSONCodec">encoding.ProtoJSONCodec</a> for (de)serialisation
</p>
</section>

  <h3 tabindex="-1" id="pkg-functions" class="Documentation-functionsHeader">Functions <a href="#pkg-functions" title="Go to Functions" aria-label="Go to Functions">¶</a></h3>

  <section class="Documentation-functions"><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="CancelInvocation" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/rpc.go#L124">CancelInvocation</a> <a class="Documentation-idLink" href="#CancelInvocation" title="Go to CancelInvocation" aria-label="Go to CancelInvocation">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.16.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func CancelInvocation(ctx <a href="#Context">Context</a>, invocationId <a href="/builtin#string">string</a>)</pre>
    </div>
  <p>CancelInvocation cancels the invocation with the given invocationId.
For more info about cancellations, see <a href="https://docs.restate.dev/operate/invocation/#cancelling-invocations">https://docs.restate.dev/operate/invocation/#cancelling-invocations</a>
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="Clear" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/state.go#L33">Clear</a> <a class="Documentation-idLink" href="#Clear" title="Go to Clear" aria-label="Go to Clear">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.11.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func Clear(ctx <a href="#ObjectContext">ObjectContext</a>, key <a href="/builtin#string">string</a>)</pre>
    </div>
  <p>Clear deletes a key
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="ClearAll" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/state.go#L38">ClearAll</a> <a class="Documentation-idLink" href="#ClearAll" title="Go to ClearAll" aria-label="Go to ClearAll">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.11.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func ClearAll(ctx <a href="#ObjectContext">ObjectContext</a>)</pre>
    </div>
  <p>ClearAll drops all stored state associated with this Object key
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="IsRetryableError" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/error.go#L96">IsRetryableError</a> <a class="Documentation-idLink" href="#IsRetryableError" title="Go to IsRetryableError" aria-label="Go to IsRetryableError">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.25.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func IsRetryableError(err <a href="/builtin#error">error</a>) <a href="/builtin#bool">bool</a></pre>
    </div>
  <p>IsRetryableError reports whether err is, or wraps, a <a href="#RetryableError">RetryableError</a>.
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="IsTerminalError" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/error.go#L59">IsTerminalError</a> <a class="Documentation-idLink" href="#IsTerminalError" title="Go to IsTerminalError" aria-label="Go to IsTerminalError">¶</a></span>
  <span class="Documentation-sinceVersion">
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func IsTerminalError(err <a href="/builtin#error">error</a>) <a href="/builtin#bool">bool</a></pre>
    </div>
  <p>IsTerminalError reports whether err is, or wraps, a <a href="#TerminalError">TerminalError</a> - ie, that
returning it in a handler or Run function will finish the invocation with the
error as a result.
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="Key" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/context.go#L52">Key</a> <a class="Documentation-idLink" href="#Key" title="Go to Key" aria-label="Go to Key">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.11.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func Key(ctx <a href="#ObjectSharedContext">ObjectSharedContext</a>) <a href="/builtin#string">string</a></pre>
    </div>
  <p>Key retrieves the key for this virtual object invocation or this workflow invocation.
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="KillOnMaxAttempts" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/retry_options.go#L131">KillOnMaxAttempts</a> <a class="Documentation-idLink" href="#KillOnMaxAttempts" title="Go to KillOnMaxAttempts" aria-label="Go to KillOnMaxAttempts">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.20.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func KillOnMaxAttempts() withOnMaxAttempts</pre>
    </div>
  <p>KillOnMaxAttempts kills the invocation when the maximum number of attempts is reached.
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="NewObject" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/service_definition.go#L106">NewObject</a> <a class="Documentation-idLink" href="#NewObject" title="Go to NewObject" aria-label="Go to NewObject">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.10.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func NewObject(name <a href="/builtin#string">string</a>, opts ...<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#ServiceDefinitionOption">ServiceDefinitionOption</a>) *object</pre>
    </div>
  <p>NewObject creates a new named Virtual Object
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="NewObjectHandler" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/handler.go#L110">NewObjectHandler</a> <a class="Documentation-idLink" href="#NewObjectHandler" title="Go to NewObjectHandler" aria-label="Go to NewObjectHandler">¶</a></span>
  <span class="Documentation-sinceVersion">
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func NewObjectHandler[I <a href="/builtin#any">any</a>, O <a href="/builtin#any">any</a>](fn <a href="#ObjectHandlerFn">ObjectHandlerFn</a>[I, O], opts ...<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#HandlerOption">HandlerOption</a>) *objectHandler[I, O]</pre>
    </div>
  <p>NewObjectHandler converts a function of signature <a href="#ObjectHandlerFn">ObjectHandlerFn</a> into an exclusive-mode handler on a Virtual Object.
The handler will have access to a full <a href="#ObjectContext">ObjectContext</a> which may mutate state.
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="NewObjectSharedHandler" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/handler.go#L124">NewObjectSharedHandler</a> <a class="Documentation-idLink" href="#NewObjectSharedHandler" title="Go to NewObjectSharedHandler" aria-label="Go to NewObjectSharedHandler">¶</a></span>
  <span class="Documentation-sinceVersion">
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func NewObjectSharedHandler[I <a href="/builtin#any">any</a>, O <a href="/builtin#any">any</a>](fn <a href="#ObjectSharedHandlerFn">ObjectSharedHandlerFn</a>[I, O], opts ...<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#HandlerOption">HandlerOption</a>) *objectHandler[I, O]</pre>
    </div>
  <p>NewObjectSharedHandler converts a function of signature <a href="#ObjectSharedHandlerFn">ObjectSharedHandlerFn</a> into a shared-mode handler on a Virtual Object.
The handler will only have access to a <a href="#ObjectSharedContext">ObjectSharedContext</a> which can only read a snapshot of state.
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="NewService" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/service_definition.go#L68">NewService</a> <a class="Documentation-idLink" href="#NewService" title="Go to NewService" aria-label="Go to NewService">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.10.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func NewService(name <a href="/builtin#string">string</a>, opts ...<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#ServiceDefinitionOption">ServiceDefinitionOption</a>) *service</pre>
    </div>
  <p>NewService creates a new named Service
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="NewServiceHandler" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/handler.go#L46">NewServiceHandler</a> <a class="Documentation-idLink" href="#NewServiceHandler" title="Go to NewServiceHandler" aria-label="Go to NewServiceHandler">¶</a></span>
  <span class="Documentation-sinceVersion">
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func NewServiceHandler[I <a href="/builtin#any">any</a>, O <a href="/builtin#any">any</a>](fn <a href="#ServiceHandlerFn">ServiceHandlerFn</a>[I, O], opts ...<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#HandlerOption">HandlerOption</a>) *serviceHandler[I, O]</pre>
    </div>
  <p>NewServiceHandler converts a function of signature <a href="#ServiceHandlerFn">ServiceHandlerFn</a> into a handler on a Restate service.
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="NewWorkflow" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/service_definition.go#L144">NewWorkflow</a> <a class="Documentation-idLink" href="#NewWorkflow" title="Go to NewWorkflow" aria-label="Go to NewWorkflow">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.12.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func NewWorkflow(name <a href="/builtin#string">string</a>, opts ...<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#ServiceDefinitionOption">ServiceDefinitionOption</a>) *workflow</pre>
    </div>
  <p>NewWorkflow creates a new named Workflow
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="NewWorkflowHandler" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/handler.go#L211">NewWorkflowHandler</a> <a class="Documentation-idLink" href="#NewWorkflowHandler" title="Go to NewWorkflowHandler" aria-label="Go to NewWorkflowHandler">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.12.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func NewWorkflowHandler[I <a href="/builtin#any">any</a>, O <a href="/builtin#any">any</a>](fn <a href="#WorkflowHandlerFn">WorkflowHandlerFn</a>[I, O], opts ...<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#HandlerOption">HandlerOption</a>) *workflowHandler[I, O]</pre>
    </div>
  <p>NewWorkflowHandler converts a function of signature <a href="#WorkflowHandlerFn">WorkflowHandlerFn</a> into the &#39;Run&#39; handler on a Workflow.
The handler will have access to a full <a href="#WorkflowContext">WorkflowContext</a> which may mutate state.
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="NewWorkflowSharedHandler" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/handler.go#L225">NewWorkflowSharedHandler</a> <a class="Documentation-idLink" href="#NewWorkflowSharedHandler" title="Go to NewWorkflowSharedHandler" aria-label="Go to NewWorkflowSharedHandler">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.12.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func NewWorkflowSharedHandler[I <a href="/builtin#any">any</a>, O <a href="/builtin#any">any</a>](fn <a href="#WorkflowSharedHandlerFn">WorkflowSharedHandlerFn</a>[I, O], opts ...<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#HandlerOption">HandlerOption</a>) *workflowHandler[I, O]</pre>
    </div>
  <p>NewWorkflowSharedHandler converts a function of signature <a href="#ObjectSharedHandlerFn">ObjectSharedHandlerFn</a> into a shared-mode handler on a Workflow.
The handler will only have access to a <a href="#WorkflowSharedContext">WorkflowSharedContext</a> which can only read a snapshot of state.
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="PauseOnMaxAttempts" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/retry_options.go#L128">PauseOnMaxAttempts</a> <a class="Documentation-idLink" href="#PauseOnMaxAttempts" title="Go to PauseOnMaxAttempts" aria-label="Go to PauseOnMaxAttempts">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.20.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func PauseOnMaxAttempts() withOnMaxAttempts</pre>
    </div>
  <p>PauseOnMaxAttempts pauses the invocation when the maximum number of attempts is reached.
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="Rand" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/random.go#L12">Rand</a> <a class="Documentation-idLink" href="#Rand" title="Go to Rand" aria-label="Go to Rand">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.11.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func Rand(ctx <a href="#Context">Context</a>) *<a href="/math/rand/v2">rand2</a>.<a href="/math/rand/v2#Rand">Rand</a></pre>
    </div>
  <p>Rand returns a random source which will give deterministic results for a given invocation
</p><p>This rand instance is not safe for use inside .Run()
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="RandSource" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/random.go#L28">RandSource</a> <a class="Documentation-idLink" href="#RandSource" title="Go to RandSource" aria-label="Go to RandSource">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.21.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func RandSource(ctx <a href="#Context">Context</a>) <a href="/math/rand/v2">rand2</a>.<a href="/math/rand/v2#Source">Source</a></pre>
    </div>
  <p>RandSource returns a random source to be used with math/rand/v2 implementations.
</p><p>To create a random implementation, use `rand2.New(RandSource(ctx))`
</p><p>This source instance is not safe for use inside .Run()
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="RejectAwakeable" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/awakeables.go#L41">RejectAwakeable</a> <a class="Documentation-idLink" href="#RejectAwakeable" title="Go to RejectAwakeable" aria-label="Go to RejectAwakeable">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.11.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func RejectAwakeable(ctx <a href="#Context">Context</a>, id <a href="/builtin#string">string</a>, reason <a href="/builtin#error">error</a>)</pre>
    </div>
  <p>RejectAwakeable allows an awakeable (not necessarily from this service) to be
rejected with a particular error.
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="RejectSignal" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/signals.go#L36">RejectSignal</a> <a class="Documentation-idLink" href="#RejectSignal" title="Go to RejectSignal" aria-label="Go to RejectSignal">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.25.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func RejectSignal(ctx <a href="#Context">Context</a>, invocationID <a href="/builtin#string">string</a>, name <a href="/builtin#string">string</a>, reason <a href="/builtin#error">error</a>)</pre>
    </div>
  <p>RejectSignal rejects a signal on an invocation with a particular error.
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="ResolveAwakeable" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/awakeables.go#L35">ResolveAwakeable</a> <a class="Documentation-idLink" href="#ResolveAwakeable" title="Go to ResolveAwakeable" aria-label="Go to ResolveAwakeable">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.11.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func ResolveAwakeable[T <a href="/builtin#any">any</a>](ctx <a href="#Context">Context</a>, id <a href="/builtin#string">string</a>, value T, options ...<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#ResolveAwakeableOption">ResolveAwakeableOption</a>)</pre>
    </div>
  <p>ResolveAwakeable allows an awakeable (not necessarily from this service) to be
resolved with a particular value.
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="ResolveSignal" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/signals.go#L31">ResolveSignal</a> <a class="Documentation-idLink" href="#ResolveSignal" title="Go to ResolveSignal" aria-label="Go to ResolveSignal">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.25.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func ResolveSignal[T <a href="/builtin#any">any</a>](ctx <a href="#Context">Context</a>, invocationID <a href="/builtin#string">string</a>, name <a href="/builtin#string">string</a>, value T, options ...<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#ResolveSignalOption">ResolveSignalOption</a>)</pre>
    </div>
  <p>ResolveSignal resolves a signal on an invocation with a particular value.
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="Set" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/state.go#L28">Set</a> <a class="Documentation-idLink" href="#Set" title="Go to Set" aria-label="Go to Set">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.11.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func Set[T <a href="/builtin#any">any</a>](ctx <a href="#ObjectContext">ObjectContext</a>, key <a href="/builtin#string">string</a>, value T, options ...<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#SetOption">SetOption</a>)</pre>
    </div>
  <p>Set sets a value against a key, using the provided codec (defaults to JSON)
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="UUID" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/random.go#L19">UUID</a> <a class="Documentation-idLink" href="#UUID" title="Go to UUID" aria-label="Go to UUID">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.21.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func UUID(ctx <a href="#Context">Context</a>) <a href="/github.com/google/uuid">uuid</a>.<a href="/github.com/google/uuid#UUID">UUID</a></pre>
    </div>
  <p>UUID returns a random UUID seeded deterministically for a given invocation.
</p><p>This method should not be used inside .Run()
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="Wait" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/futures.go#L69">Wait</a> <a class="Documentation-idLink" href="#Wait" title="Go to Wait" aria-label="Go to Wait">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.21.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func Wait(ctx <a href="#Context">Context</a>, futs ...<a href="#Future">Future</a>) <a href="/iter">iter</a>.<a href="/iter#Seq2">Seq2</a>[<a href="#Future">Future</a>, <a href="#TerminalError">TerminalError</a>]</pre>
    </div>
  <p>Wait returns an iterator that yields Futures as they complete in order of completion.
The iterator continues until all Futures have completed or a cancellation error occurs.
If a cancellation error occurs, it is yielded as the final element with a nil Future.
</p><p>Example:
</p><pre>func MyHandler(ctx restate.Context, input string) ([]string, error) {
	fut1 := restate.Service[string](ctx, &#34;service1&#34;, &#34;method1&#34;).RequestFuture(input)
	fut2 := restate.Service[string](ctx, &#34;service2&#34;, &#34;method2&#34;).RequestFuture(input)
	fut3 := restate.Service[string](ctx, &#34;service3&#34;, &#34;method3&#34;).RequestFuture(input)

	results := []string{}
	for fut, err := range restate.Wait(ctx, fut1, fut2, fut3) {
		if err != nil {
			return nil, err
		}
		result, err := fut.(restate.ResponseFuture[string]).Response()
		if err != nil {
			return nil, err
		}
		results = append(results, result)
	}
	return results, nil
}
</pre>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="WaitFirst" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/futures.go#L38">WaitFirst</a> <a class="Documentation-idLink" href="#WaitFirst" title="Go to WaitFirst" aria-label="Go to WaitFirst">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.21.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func WaitFirst(ctx <a href="#Context">Context</a>, futs ...<a href="#Future">Future</a>) (resultFut <a href="#Future">Future</a>, cancellationError <a href="#TerminalError">TerminalError</a>)</pre>
    </div>
  <p>WaitFirst waits for the first Future to complete among the provided Futures and returns it.
If the invocation is canceled, a cancellation error is returned.
</p><p>Example:
</p><pre>func MyHandler(ctx restate.Context, input string) (string, error) {
	fut1 := restate.Service[string](ctx, &#34;service1&#34;, &#34;method1&#34;).RequestFuture(input)
	fut2 := restate.After(ctx, 5 * time.Second)
	fut3 := restate.Service[string](ctx, &#34;service2&#34;, &#34;method2&#34;).RequestFuture(input)

	firstComplete, err := restate.WaitFirst(ctx, fut1, fut2, fut3)
	if err != nil {
		return &#34;&#34;, err
	}
	// Handle the first completed future
	switch firstComplete {
	case fut1:
		return fut1.Response()
	case fut2:
		return &#34;&#34;, fmt.Errorf(&#34;timeout&#34;)
	case fut3:
		return fut3.Response()
	default:
		return &#34;&#34;, fmt.Errorf(&#34;unknown future&#34;)
	}
}
</pre>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="WithAbortTimeout" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/invocation_options.go#L72">WithAbortTimeout</a> <a class="Documentation-idLink" href="#WithAbortTimeout" title="Go to WithAbortTimeout" aria-label="Go to WithAbortTimeout">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.18.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func WithAbortTimeout(abortTimeout <a href="/time">time</a>.<a href="/time#Duration">Duration</a>) withAbortTimeout</pre>
    </div>
  <p>WithAbortTimeout sets the abort timeout duration for a service/handler.
</p><p>This timer guards against stalled service/handler invocations that are supposed to terminate. The
abort timeout is started after the inactivity timeout has expired and the service/handler
invocation has been asked to gracefully terminate. Once the timer expires, it will abort the
service/handler invocation.
</p><p>This timer potentially *interrupts* user code. If the user code needs longer to gracefully
terminate, then this value needs to be set accordingly.
</p><p>This overrides the default abort timeout configured in the restate-server for all invocations to
this service.
</p><p>NOTE: You can set this field only if you register this service against restate-server &gt;= 1.4,
otherwise the service discovery will fail.
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="WithCodec" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/codec.go#L77">WithCodec</a> <a class="Documentation-idLink" href="#WithCodec" title="Go to WithCodec" aria-label="Go to WithCodec">¶</a></span>
  <span class="Documentation-sinceVersion">
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func WithCodec(codec <a href="/github.com/restatedev/sdk-go@v1.1.0/encoding">encoding</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/encoding#Codec">Codec</a>) withCodec</pre>
    </div>
  <p>WithCodec sets the <a href="/github.com/restatedev/sdk-go@v1.1.0/encoding#Codec">encoding.Codec</a> used to (de)serialise values. It applies to any
operation that (de)serialises; on handlers and calls it sets both the input and output
codec (override one with <a href="#WithInputCodec">WithInputCodec</a> / <a href="#WithOutputCodec">WithOutputCodec</a>).
</p><p>See also <a href="#WithProto">WithProto</a>, <a href="#WithBinary">WithBinary</a>, <a href="#WithJSON">WithJSON</a>, <a href="#WithProtoJSON">WithProtoJSON</a>.
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="WithDelay" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/rpc.go#L268">WithDelay</a> <a class="Documentation-idLink" href="#WithDelay" title="Go to WithDelay" aria-label="Go to WithDelay">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.11.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func WithDelay(delay <a href="/time">time</a>.<a href="/time#Duration">Duration</a>) withDelay</pre>
    </div>
  <p>WithDelay is an <a href="#SendOption">SendOption</a> to specify the duration to delay the request
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="WithDocumentation" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/invocation_options.go#L38">WithDocumentation</a> <a class="Documentation-idLink" href="#WithDocumentation" title="Go to WithDocumentation" aria-label="Go to WithDocumentation">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.16.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func WithDocumentation(documentation <a href="/builtin#string">string</a>) withDocumentation</pre>
    </div>
  <p>WithDocumentation sets the handler/service documentation, shown in the UI and other Restate observability tools.
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="WithEnableLazyState" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/invocation_options.go#L98">WithEnableLazyState</a> <a class="Documentation-idLink" href="#WithEnableLazyState" title="Go to WithEnableLazyState" aria-label="Go to WithEnableLazyState">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.18.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func WithEnableLazyState(enableLazyState <a href="/builtin#bool">bool</a>) withEnableLazyState</pre>
    </div>
  <p>WithEnableLazyState enables or disables lazy state for a service/handler.
</p><p>When set to true, lazy state will be enabled for all invocations to this service/handler. This is
relevant only for workflows and virtual objects.
</p><p>NOTE: You can set this field only if you register this service against restate-server &gt;= 1.4,
otherwise the service discovery will fail.
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="WithHeaders" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/rpc.go#L188">WithHeaders</a> <a class="Documentation-idLink" href="#WithHeaders" title="Go to WithHeaders" aria-label="Go to WithHeaders">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.10.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func WithHeaders(headers map[<a href="/builtin#string">string</a>]<a href="/builtin#string">string</a>) withHeaders</pre>
    </div>
  <p>WithHeaders is an option to specify outgoing headers when making a call
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="WithIdempotencyKey" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/rpc.go#L218">WithIdempotencyKey</a> <a class="Documentation-idLink" href="#WithIdempotencyKey" title="Go to WithIdempotencyKey" aria-label="Go to WithIdempotencyKey">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.16.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func WithIdempotencyKey(idempotencyKey <a href="/builtin#string">string</a>) withIdempotencyKey</pre>
    </div>
  <p>WithIdempotencyKey is an option to specify the idempotency key to set when making a call
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="WithIdempotencyRetention" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/invocation_options.go#L121">WithIdempotencyRetention</a> <a class="Documentation-idLink" href="#WithIdempotencyRetention" title="Go to WithIdempotencyRetention" aria-label="Go to WithIdempotencyRetention">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.18.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func WithIdempotencyRetention(idempotencyRetention <a href="/time">time</a>.<a href="/time#Duration">Duration</a>) withIdempotencyRetention</pre>
    </div>
  <p>WithIdempotencyRetention sets the idempotency retention duration for a service/handler.
</p><p>NOTE: You can set this field only if you register this service against restate-server &gt;= 1.4,
otherwise the service discovery will fail.
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="WithInactivityTimeout" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/invocation_options.go#L153">WithInactivityTimeout</a> <a class="Documentation-idLink" href="#WithInactivityTimeout" title="Go to WithInactivityTimeout" aria-label="Go to WithInactivityTimeout">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.18.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func WithInactivityTimeout(inactivityTimeout <a href="/time">time</a>.<a href="/time#Duration">Duration</a>) withInactivityTimeout</pre>
    </div>
  <p>WithInactivityTimeout sets the inactivity timeout duration for a service/handler.
</p><p>This timer guards against stalled invocations. Once it expires, Restate triggers a graceful
termination by asking the invocation to suspend (which preserves intermediate progress).
</p><p>The abort timeout is used to abort the invocation, in case it doesn&#39;t react to the request to
suspend.
</p><p>This overrides the default inactivity timeout configured in the restate-server for all
invocations to this service.
</p><p>NOTE: You can set this field only if you register this service against restate-server &gt;= 1.4,
otherwise the service discovery will fail.
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="WithIngressPrivate" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/invocation_options.go#L179">WithIngressPrivate</a> <a class="Documentation-idLink" href="#WithIngressPrivate" title="Go to WithIngressPrivate" aria-label="Go to WithIngressPrivate">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.18.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func WithIngressPrivate(ingressPrivate <a href="/builtin#bool">bool</a>) withIngressPrivate</pre>
    </div>
  <p>WithIngressPrivate sets whether the service/handler is private (not accessible from HTTP or Kafka ingress).
</p><p>When set to true this service/handler cannot be invoked from the restate-server
HTTP and Kafka ingress, but only from other services.
</p><p>NOTE: You can set this field only if you register this service against restate-server &gt;= 1.4,
otherwise the service discovery will fail.
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="WithInitialRetryInterval" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/retry_options.go#L55">WithInitialRetryInterval</a> <a class="Documentation-idLink" href="#WithInitialRetryInterval" title="Go to WithInitialRetryInterval" aria-label="Go to WithInitialRetryInterval">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.16.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func WithInitialRetryInterval(d <a href="/time">time</a>.<a href="/time#Duration">Duration</a>) withInitialRetryInterval</pre>
    </div>
  <p>WithInitialRetryInterval sets the delay before the first retry attempt. The interval
then grows by the factor set with <a href="#WithRetryIntervalFactor">WithRetryIntervalFactor</a>, capped by
<a href="#WithMaxRetryInterval">WithMaxRetryInterval</a>.
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="WithInputCodec" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/codec.go#L99">WithInputCodec</a> <a class="Documentation-idLink" href="#WithInputCodec" title="Go to WithInputCodec" aria-label="Go to WithInputCodec">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.25.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func WithInputCodec(codec <a href="/github.com/restatedev/sdk-go@v1.1.0/encoding">encoding</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/encoding#Codec">Codec</a>) withInputCodec</pre>
    </div>
  <p>WithInputCodec sets the <a href="/github.com/restatedev/sdk-go@v1.1.0/encoding#Codec">encoding.Codec</a> used to (de)serialise the input of a handler
or call, independently of the output.
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="WithInvocationRetryPolicy" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/invocation_options.go#L264">WithInvocationRetryPolicy</a> <a class="Documentation-idLink" href="#WithInvocationRetryPolicy" title="Go to WithInvocationRetryPolicy" aria-label="Go to WithInvocationRetryPolicy">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.20.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func WithInvocationRetryPolicy(opts ...<a href="#InvocationRetryPolicyOption">InvocationRetryPolicyOption</a>) withInvocationRetryPolicy</pre>
    </div>
  <p>WithInvocationRetryPolicy sets the invocation retry policy used by Restate when invoking this service/handler.
</p><p>NOTE: You can set this field only if you register this service against restate-server &gt;= 1.5,
otherwise the service discovery will fail.
</p><p>Unset fields inherit server defaults. The policy controls an exponential backoff with optional capping and a terminal action:
</p><ul class="Documentation-bulletList">
  <li>initial interval before the first retry attempt</li>
  <li>exponentiation factor to compute the next retry delay</li>
  <li>maximum interval cap</li>
  <li>maximum attempts (initial call counts as the first attempt)</li>
  <li>behavior when max attempts is reached (OnMaxAttempts: PAUSE | KILL)</li>
</ul>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="WithJournalRetention" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/invocation_options.go#L207">WithJournalRetention</a> <a class="Documentation-idLink" href="#WithJournalRetention" title="Go to WithJournalRetention" aria-label="Go to WithJournalRetention">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.18.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func WithJournalRetention(journalRetention <a href="/time">time</a>.<a href="/time#Duration">Duration</a>) withJournalRetention</pre>
    </div>
  <p>WithJournalRetention sets the journal retention duration for a service/handler.
</p><p>The journal retention for invocations to this service/handler.
</p><p>In case the request has an idempotency key, the idempotency retention caps the journal retention
time.
</p><p>NOTE: You can set this field only if you register this service against restate-server &gt;= 1.4,
otherwise the service discovery will fail.
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="WithLimitKey" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/rpc.go#L248">WithLimitKey</a> <a class="Documentation-idLink" href="#WithLimitKey" title="Go to WithLimitKey" aria-label="Go to WithLimitKey">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.25.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func WithLimitKey(limitKey <a href="/builtin#string">string</a>) withLimitKey</pre>
    </div>
  <p>WithLimitKey sets the concurrency limit key when making a call.
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="WithMaxRetryAttempts" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/retry_options.go#L31">WithMaxRetryAttempts</a> <a class="Documentation-idLink" href="#WithMaxRetryAttempts" title="Go to WithMaxRetryAttempts" aria-label="Go to WithMaxRetryAttempts">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.16.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func WithMaxRetryAttempts(maxAttempts <a href="/builtin#uint">uint</a>) withMaxRetryAttempts</pre>
    </div>
  <p>WithMaxRetryAttempts sets the maximum number of attempts before giving up retrying.
The initial call counts as the first attempt.
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="WithMaxRetryDuration" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/retry_options.go#L114">WithMaxRetryDuration</a> <a class="Documentation-idLink" href="#WithMaxRetryDuration" title="Go to WithMaxRetryDuration" aria-label="Go to WithMaxRetryDuration">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.16.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func WithMaxRetryDuration(d <a href="/time">time</a>.<a href="/time#Duration">Duration</a>) withMaxRetryDuration</pre>
    </div>
  <p>WithMaxRetryDuration sets the maximum total time spent retrying before giving up.
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="WithMaxRetryInterval" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/retry_options.go#L77">WithMaxRetryInterval</a> <a class="Documentation-idLink" href="#WithMaxRetryInterval" title="Go to WithMaxRetryInterval" aria-label="Go to WithMaxRetryInterval">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.16.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func WithMaxRetryInterval(d <a href="/time">time</a>.<a href="/time#Duration">Duration</a>) withMaxRetryInterval</pre>
    </div>
  <p>WithMaxRetryInterval caps the delay between retry attempts.
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="WithMetadata" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/options.go#L21">WithMetadata</a> <a class="Documentation-idLink" href="#WithMetadata" title="Go to WithMetadata" aria-label="Go to WithMetadata">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.16.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func WithMetadata(metadataKey <a href="/builtin#string">string</a>, metadataValue <a href="/builtin#string">string</a>) <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/errors">errors</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/errors#MetadataOption">MetadataOption</a></pre>
    </div>
  <p>WithMetadata adds the given key/value as metadata. It applies anywhere metadata is
accepted: service/handler definitions (shown in the Admin API) and <a href="#ToTerminalError">ToTerminalError</a>.
Multiple metadata options merge.
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="WithMetadataMap" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/options.go#L14">WithMetadataMap</a> <a class="Documentation-idLink" href="#WithMetadataMap" title="Go to WithMetadataMap" aria-label="Go to WithMetadataMap">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.16.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func WithMetadataMap(metadata map[<a href="/builtin#string">string</a>]<a href="/builtin#string">string</a>) <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/errors">errors</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/errors#MetadataOption">MetadataOption</a></pre>
    </div>
  <p>WithMetadataMap adds the given metadata. It applies anywhere metadata is accepted:
service/handler definitions (shown in the Admin API) and <a href="#ToTerminalError">ToTerminalError</a>. Multiple
metadata options merge.
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="WithMockContext" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/context.go#L75">WithMockContext</a> <a class="Documentation-idLink" href="#WithMockContext" title="Go to WithMockContext" aria-label="Go to WithMockContext">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.15.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func WithMockContext(ctx <a href="#MockableContext">MockableContext</a>) ctxWrapper</pre>
    </div>
  <p>WithMockContext wraps a <a href="#MockableContext">MockableContext</a>. To be used with *MockContext from the
github.com/restatedev/sdk-go/x/mocks module.
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="WithName" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/options.go#L41">WithName</a> <a class="Documentation-idLink" href="#WithName" title="Go to WithName" aria-label="Go to WithName">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.16.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func WithName(name <a href="/builtin#string">string</a>) withName</pre>
    </div>
  <p>WithName sets the operation name, shown in the UI and other Restate observability tools.
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="WithOutputCodec" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/codec.go#L121">WithOutputCodec</a> <a class="Documentation-idLink" href="#WithOutputCodec" title="Go to WithOutputCodec" aria-label="Go to WithOutputCodec">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.25.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func WithOutputCodec(codec <a href="/github.com/restatedev/sdk-go@v1.1.0/encoding">encoding</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/encoding#Codec">Codec</a>) withOutputCodec</pre>
    </div>
  <p>WithOutputCodec sets the <a href="/github.com/restatedev/sdk-go@v1.1.0/encoding#Codec">encoding.Codec</a> used to (de)serialise the output of a handler
or call, independently of the input.
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="WithRetryIntervalFactor" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/retry_options.go#L100">WithRetryIntervalFactor</a> <a class="Documentation-idLink" href="#WithRetryIntervalFactor" title="Go to WithRetryIntervalFactor" aria-label="Go to WithRetryIntervalFactor">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.16.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func WithRetryIntervalFactor(f <a href="/builtin#float32">float32</a>) withRetryIntervalFactor</pre>
    </div>
  <p>WithRetryIntervalFactor sets the multiplier applied to the retry interval after each
attempt (e.g. 2 doubles the interval each time).
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="WithScope" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/rpc.go#L158">WithScope</a> <a class="Documentation-idLink" href="#WithScope" title="Go to WithScope" aria-label="Go to WithScope">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.25.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func WithScope(scope <a href="/builtin#string">string</a>) withScope</pre>
    </div>
  <p>WithScope sets the scope within which invocations made through this client are routed.
</p><p>It is a client-level option: pass it when constructing a client (e.g. via <a href="#Service">Service</a>,
<a href="#Object">Object</a> or <a href="#Workflow">Workflow</a>, or the equivalent ingress constructors) and it applies to
every Request, RequestFuture and Send made through that client. An empty scope is a
no-op, leaving the invocation unscoped.
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="WithValue" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/context.go#L65">WithValue</a> <a class="Documentation-idLink" href="#WithValue" title="Go to WithValue" aria-label="Go to WithValue">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.23.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func WithValue[T <a href="#Context">Context</a>](restateCtx T, key, val <a href="/builtin#any">any</a>) T</pre>
    </div>
  <p>WithValue is like context.WithValue, but wrapping the restate context
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="WithWorkflowRetention" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/invocation_options.go#L234">WithWorkflowRetention</a> <a class="Documentation-idLink" href="#WithWorkflowRetention" title="Go to WithWorkflowRetention" aria-label="Go to WithWorkflowRetention">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.18.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func WithWorkflowRetention(workflowCompletionRetention <a href="/time">time</a>.<a href="/time#Duration">Duration</a>) withWorkflowRetention</pre>
    </div>
  <p>WithWorkflowRetention sets the workflow completion retention duration for a handler.
</p><p>The retention duration for this workflow handler.
</p><p>This is only valid when HandlerType == WORKFLOW.
</p><p>NOTE: You can set this field only if you register this service against restate-server &gt;= 1.4,
otherwise the service discovery will fail.
</p>

  

        </div><div class="Documentation-function">
	  
  
  
    <h4 tabindex="-1" id="WrapContext" data-kind="function" class="Documentation-functionHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/context.go#L58">WrapContext</a> <a class="Documentation-idLink" href="#WrapContext" title="Go to WrapContext" aria-label="Go to WrapContext">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.23.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func WrapContext[T <a href="#Context">Context</a>](restateCtx T, wrappedCtx <a href="/context">context</a>.<a href="/context#Context">Context</a>) T</pre>
    </div>
  <p>WrapContext wraps the provided Restate context with a context.Context,
making sure all Context.Values from the wrappedCtx are accessible from the Restate context.
</p>

  

        </div></section>

  <h3 tabindex="-1" id="pkg-types" class="Documentation-typesHeader">Types <a href="#pkg-types" title="Go to Types" aria-label="Go to Types">¶</a></h3>

  <section class="Documentation-types"><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="AfterFuture" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/timers.go#L27">AfterFuture</a> <a class="Documentation-idLink" href="#AfterFuture" title="Go to AfterFuture" aria-label="Go to AfterFuture">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.11.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type AfterFuture = <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/restatecontext">restatecontext</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/restatecontext#AfterFuture">AfterFuture</a></pre>
    </div>
  <p>AfterFuture is returned by the After operation which allows you to do other work concurrently
with the sleep.
</p>
<div class="Documentation-typeFunc">
    
  
  
    <h4 tabindex="-1" id="After" data-kind="function" class="Documentation-typeFuncHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/timers.go#L21">After</a> <a class="Documentation-idLink" href="#After" title="Go to After" aria-label="Go to After">¶</a></span>
  <span class="Documentation-sinceVersion">
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func After(ctx <a href="#Context">Context</a>, d <a href="/time">time</a>.<a href="/time#Duration">Duration</a>, opts ...<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#SleepOption">SleepOption</a>) <a href="#AfterFuture">AfterFuture</a></pre>
    </div>
  <p>After is an alternative to <a href="#Sleep">Sleep</a> which allows you to complete other tasks concurrently
with the sleep. This is particularly useful when combined with <a href="#WaitFirst">WaitFirst</a> to race between
the sleep and other <a href="#Future">Future</a> operations.
</p>

  

  </div>
  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="AttachFuture" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/rpc.go#L129">AttachFuture</a> <a class="Documentation-idLink" href="#AttachFuture" title="Go to AttachFuture" aria-label="Go to AttachFuture">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.16.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type AttachFuture[O <a href="/builtin#any">any</a>] interface {
<span id="AttachFuture.Response" data-kind="method">	<span class="comment">// Response blocks on the response to the call and returns it or the associated error</span>
</span>	<span class="comment">// It is *not* safe to call this in a goroutine - use Context.Select if you</span>
	<span class="comment">// want to wait on multiple results at once.</span>
	Response() (O, <a href="#TerminalError">TerminalError</a>)
	<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/restatecontext">restatecontext</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/restatecontext#Future">Future</a>
}</pre>
    </div>
  <p>AttachFuture is a handle on a potentially not-yet completed call.
</p>
<div class="Documentation-typeFunc">
    
  
  
    <h4 tabindex="-1" id="AttachInvocation" data-kind="function" class="Documentation-typeFuncHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/rpc.go#L138">AttachInvocation</a> <a class="Documentation-idLink" href="#AttachInvocation" title="Go to AttachInvocation" aria-label="Go to AttachInvocation">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.16.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func AttachInvocation[T <a href="/builtin#any">any</a>](ctx <a href="#Context">Context</a>, invocationId <a href="/builtin#string">string</a>, options ...<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#AttachOption">AttachOption</a>) <a href="#AttachFuture">AttachFuture</a>[T]</pre>
    </div>
  <p>AttachInvocation attaches to the invocation with the given invocation id.
</p>

  

  </div>
  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="AttachOption" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/rpc.go#L22">AttachOption</a> <a class="Documentation-idLink" href="#AttachOption" title="Go to AttachOption" aria-label="Go to AttachOption">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.19.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type AttachOption = <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#AttachOption">AttachOption</a></pre>
    </div>
  <p>AttachOption is an option for <a href="#AttachInvocation">AttachInvocation</a>.
</p>

  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="AwakeableFuture" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/awakeables.go#L22">AwakeableFuture</a> <a class="Documentation-idLink" href="#AwakeableFuture" title="Go to AwakeableFuture" aria-label="Go to AwakeableFuture">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.11.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type AwakeableFuture[T <a href="/builtin#any">any</a>] interface {
<span id="AwakeableFuture.Id" data-kind="method">	<span class="comment">// Id returns the awakeable ID, which can be stored or sent to a another service</span>
</span>	Id() <a href="/builtin#string">string</a>
<span id="AwakeableFuture.Result" data-kind="method">	<span class="comment">// Result blocks on receiving the result of the awakeable, returning the value it was</span>
</span>	<span class="comment">// resolved or otherwise returning the error it was rejected with.</span>
	<span class="comment">// It is *not* safe to call this in a goroutine - use Context.Select if you</span>
	<span class="comment">// want to wait on multiple results at once.</span>
	Result() (T, <a href="#TerminalError">TerminalError</a>)
	<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/restatecontext">restatecontext</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/restatecontext#Future">Future</a>
}</pre>
    </div>
  <p>AwakeableFuture is a &#39;promise&#39; to a future value or error, that can be resolved or rejected by other services.
</p>
<div class="Documentation-typeFunc">
    
  
  
    <h4 tabindex="-1" id="Awakeable" data-kind="function" class="Documentation-typeFuncHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/awakeables.go#L17">Awakeable</a> <a class="Documentation-idLink" href="#Awakeable" title="Go to Awakeable" aria-label="Go to Awakeable">¶</a></span>
  <span class="Documentation-sinceVersion">
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func Awakeable[T <a href="/builtin#any">any</a>](ctx <a href="#Context">Context</a>, options ...<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#AwakeableOption">AwakeableOption</a>) <a href="#AwakeableFuture">AwakeableFuture</a>[T]</pre>
    </div>
  <p>Awakeable returns a Restate awakeable; a &#39;promise&#39; to a future
value or error, that can be resolved or rejected by other services.
</p>

  

  </div>
  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="AwakeableOption" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/awakeables.go#L10">AwakeableOption</a> <a class="Documentation-idLink" href="#AwakeableOption" title="Go to AwakeableOption" aria-label="Go to AwakeableOption">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.19.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type AwakeableOption = <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#AwakeableOption">AwakeableOption</a></pre>
    </div>
  <p>AwakeableOption is an option for <a href="#Awakeable">Awakeable</a>.
</p>

  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="Client" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/rpc.go#L55">Client</a> <a class="Documentation-idLink" href="#Client" title="Go to Client" aria-label="Go to Client">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.11.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type Client[I <a href="/builtin#any">any</a>, O <a href="/builtin#any">any</a>] interface {
<span id="Client.RequestFuture" data-kind="method">	<span class="comment">// RequestFuture makes a call and returns a handle on a future response</span>
</span>	RequestFuture(input I, options ...<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#RequestOption">RequestOption</a>) <a href="#ResponseFuture">ResponseFuture</a>[O]
<span id="Client.Request" data-kind="method">	<span class="comment">// Request makes a call and blocks on getting the response</span>
</span>	Request(input I, options ...<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#RequestOption">RequestOption</a>) (O, <a href="#TerminalError">TerminalError</a>)
	<a href="#SendClient">SendClient</a>[I]
}</pre>
    </div>
  <p>Client represents all the different ways you can invoke a particular service-method.
</p>
<div class="Documentation-typeFunc">
    
  
  
    <h4 tabindex="-1" id="Object" data-kind="function" class="Documentation-typeFuncHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/rpc.go#L35">Object</a> <a class="Documentation-idLink" href="#Object" title="Go to Object" aria-label="Go to Object">¶</a></span>
  <span class="Documentation-sinceVersion">
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func Object[O <a href="/builtin#any">any</a>](ctx <a href="#Context">Context</a>, service <a href="/builtin#string">string</a>, key <a href="/builtin#string">string</a>, method <a href="/builtin#string">string</a>, options ...<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#ClientOption">ClientOption</a>) <a href="#Client">Client</a>[<a href="/builtin#any">any</a>, O]</pre>
    </div>
  <p>Object gets an Object request client by service name, key and method name
</p>

  

  </div><div class="Documentation-typeFunc">
    
  
  
    <h4 tabindex="-1" id="Service" data-kind="function" class="Documentation-typeFuncHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/rpc.go#L25">Service</a> <a class="Documentation-idLink" href="#Service" title="Go to Service" aria-label="Go to Service">¶</a></span>
  <span class="Documentation-sinceVersion">
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func Service[O <a href="/builtin#any">any</a>](ctx <a href="#Context">Context</a>, service <a href="/builtin#string">string</a>, method <a href="/builtin#string">string</a>, options ...<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#ClientOption">ClientOption</a>) <a href="#Client">Client</a>[<a href="/builtin#any">any</a>, O]</pre>
    </div>
  <p>Service gets a Service request client by service and method name
</p>

  

  </div><div class="Documentation-typeFunc">
    
  
  
    <h4 tabindex="-1" id="WithRequestType" data-kind="function" class="Documentation-typeFuncHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/rpc.go#L93">WithRequestType</a> <a class="Documentation-idLink" href="#WithRequestType" title="Go to WithRequestType" aria-label="Go to WithRequestType">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.11.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func WithRequestType[I <a href="/builtin#any">any</a>, O <a href="/builtin#any">any</a>](inner <a href="#Client">Client</a>[<a href="/builtin#any">any</a>, O]) <a href="#Client">Client</a>[I, O]</pre>
    </div>
  <p>WithRequestType is primarily intended to be called from generated code, to provide
type safety of input types. In other contexts it&#39;s generally less cumbersome to use <a href="#Object">Object</a> and <a href="#Service">Service</a>,
as the output type can be inferred.
</p>

  

  </div><div class="Documentation-typeFunc">
    
  
  
    <h4 tabindex="-1" id="Workflow" data-kind="function" class="Documentation-typeFuncHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/rpc.go#L45">Workflow</a> <a class="Documentation-idLink" href="#Workflow" title="Go to Workflow" aria-label="Go to Workflow">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.12.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func Workflow[O <a href="/builtin#any">any</a>](ctx <a href="#Context">Context</a>, service <a href="/builtin#string">string</a>, workflowID <a href="/builtin#string">string</a>, method <a href="/builtin#string">string</a>, options ...<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#ClientOption">ClientOption</a>) <a href="#Client">Client</a>[<a href="/builtin#any">any</a>, O]</pre>
    </div>
  <p>Workflow gets a Workflow request client by service name, workflow ID and method name
</p>

  

  </div>
  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="ClientOption" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/rpc.go#L13">ClientOption</a> <a class="Documentation-idLink" href="#ClientOption" title="Go to ClientOption" aria-label="Go to ClientOption">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.11.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type ClientOption = <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#ClientOption">ClientOption</a></pre>
    </div>
  <p>ClientOption is an option for a request/send client, applied at construction
(e.g. via <a href="#Service">Service</a>, <a href="#Object">Object</a> or <a href="#Workflow">Workflow</a>).
</p>

  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="Code" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/error.go#L10">Code</a> <a class="Documentation-idLink" href="#Code" title="Go to Code" aria-label="Go to Code">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.9.1</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type Code = <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/errors">errors</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/errors#Code">Code</a></pre>
    </div>
  <p>Code is a numeric status code for an error, matching HTTP status code semantics.
</p>

  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="Context" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/context.go#L17">Context</a> <a class="Documentation-idLink" href="#Context" title="Go to Context" aria-label="Go to Context">¶</a></span>
  <span class="Documentation-sinceVersion">
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type Context interface {
	<a href="#RunContext">RunContext</a>
	<span class="comment">// contains filtered or unexported methods</span>
}</pre>
    </div>
  <p>Context is an extension of <a href="#RunContext">RunContext</a> which is passed to Restate service handlers and enables
interaction with Restate
</p>

  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="DurablePromise" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/promises.go#L17">DurablePromise</a> <a class="Documentation-idLink" href="#DurablePromise" title="Go to DurablePromise" aria-label="Go to DurablePromise">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.12.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type DurablePromise[T <a href="/builtin#any">any</a>] interface {
<span id="DurablePromise.Result" data-kind="method">	<span class="comment">// Result blocks on receiving the result of the Promise, returning the value it was</span>
</span>	<span class="comment">// resolved or otherwise returning the error it was rejected with or a cancellation error.</span>
	<span class="comment">// It is *not* safe to call this in a goroutine - use Context.Select if you</span>
	<span class="comment">// want to wait on multiple results at once.</span>
	Result() (T, <a href="#TerminalError">TerminalError</a>)
<span id="DurablePromise.Peek" data-kind="method">	<span class="comment">// Peek returns the value of the promise if it has been resolved. If it has not been resolved,</span>
</span>	<span class="comment">// the zero value of T is returned. To check explicitly for this case pass a pointer eg *string as T.</span>
	<span class="comment">// If the promise was rejected or the invocation was cancelled, an error is returned.</span>
	Peek() (T, <a href="#TerminalError">TerminalError</a>)
<span id="DurablePromise.Resolve" data-kind="method">	<span class="comment">// Resolve resolves the promise with a value, returning an error if it was already completed</span>
</span>	<span class="comment">// or if the invocation was cancelled.</span>
	Resolve(value T) <a href="#TerminalError">TerminalError</a>
<span id="DurablePromise.Reject" data-kind="method">	<span class="comment">// Reject rejects the promise with an error, returning an error if it was already completed</span>
</span>	<span class="comment">// or if the invocation was cancelled.</span>
	Reject(reason <a href="/builtin#error">error</a>) <a href="#TerminalError">TerminalError</a>
	<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/restatecontext">restatecontext</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/restatecontext#Future">Future</a>
}</pre>
    </div>
  
<div class="Documentation-typeFunc">
    
  
  
    <h4 tabindex="-1" id="Promise" data-kind="function" class="Documentation-typeFuncHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/promises.go#L13">Promise</a> <a class="Documentation-idLink" href="#Promise" title="Go to Promise" aria-label="Go to Promise">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.12.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func Promise[T <a href="/builtin#any">any</a>](ctx <a href="#WorkflowSharedContext">WorkflowSharedContext</a>, name <a href="/builtin#string">string</a>, options ...<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#PromiseOption">PromiseOption</a>) <a href="#DurablePromise">DurablePromise</a>[T]</pre>
    </div>
  <p>Promise returns a named Restate durable Promise that can be resolved or rejected during the workflow execution.
The promise is bound to the workflow and will be persisted across suspensions and retries.
</p>

  

  </div>
  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="ErrorCodeOption" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/error.go#L14">ErrorCodeOption</a> <a class="Documentation-idLink" href="#ErrorCodeOption" title="Go to ErrorCodeOption" aria-label="Go to ErrorCodeOption">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.25.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type ErrorCodeOption = <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/errors">errors</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/errors#CodeOption">CodeOption</a></pre>
    </div>
  <p>ErrorCodeOption sets the <a href="#Code">Code</a> on an error. It is shared: pass it to either
<a href="#ToTerminalError">ToTerminalError</a> or <a href="#ToRetryableError">ToRetryableError</a>.
</p>
<div class="Documentation-typeFunc">
    
  
  
    <h4 tabindex="-1" id="WithErrorCode" data-kind="function" class="Documentation-typeFuncHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/error.go#L18">WithErrorCode</a> <a class="Documentation-idLink" href="#WithErrorCode" title="Go to WithErrorCode" aria-label="Go to WithErrorCode">¶</a></span>
  <span class="Documentation-sinceVersion">
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func WithErrorCode(code <a href="#Code">Code</a>) <a href="#ErrorCodeOption">ErrorCodeOption</a></pre>
    </div>
  <p>WithErrorCode sets the <a href="#Code">Code</a> of a terminal or retryable error. Pass it to
<a href="#ToTerminalError">ToTerminalError</a> or <a href="#ToRetryableError">ToRetryableError</a>.
</p>

  

  </div>
  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="Future" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/futures.go#L10">Future</a> <a class="Documentation-idLink" href="#Future" title="Go to Future" aria-label="Go to Future">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.21.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type Future = <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/restatecontext">restatecontext</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/restatecontext#Future">Future</a></pre>
    </div>
  <p>Future is a marker interface for futures.
</p>

  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="GetOption" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/state.go#L8">GetOption</a> <a class="Documentation-idLink" href="#GetOption" title="Go to GetOption" aria-label="Go to GetOption">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.19.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type GetOption = <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#GetOption">GetOption</a></pre>
    </div>
  <p>GetOption is an option for <a href="#Get">Get</a>.
</p>

  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="HandlerOption" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/invocation_options.go#L10">HandlerOption</a> <a class="Documentation-idLink" href="#HandlerOption" title="Go to HandlerOption" aria-label="Go to HandlerOption">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.19.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type HandlerOption = <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#HandlerOption">HandlerOption</a></pre>
    </div>
  <p>HandlerOption is an option applied to a single handler at registration.
</p>

  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="Invocation" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/rpc.go#L120">Invocation</a> <a class="Documentation-idLink" href="#Invocation" title="Go to Invocation" aria-label="Go to Invocation">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.16.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type Invocation = <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/restatecontext">restatecontext</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/restatecontext#Invocation">Invocation</a></pre>
    </div>
  

  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="InvocationRetryPolicy" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/invocation_options.go#L16">InvocationRetryPolicy</a> <a class="Documentation-idLink" href="#InvocationRetryPolicy" title="Go to InvocationRetryPolicy" aria-label="Go to InvocationRetryPolicy">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.20.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type InvocationRetryPolicy = <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#InvocationRetryPolicy">InvocationRetryPolicy</a></pre>
    </div>
  <p>Retry policy types
</p>

  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="InvocationRetryPolicyOption" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/invocation_options.go#L20">InvocationRetryPolicyOption</a> <a class="Documentation-idLink" href="#InvocationRetryPolicyOption" title="Go to InvocationRetryPolicyOption" aria-label="Go to InvocationRetryPolicyOption">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.20.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type InvocationRetryPolicyOption = <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#InvocationRetryPolicyOption">InvocationRetryPolicyOption</a></pre>
    </div>
  <p>InvocationRetryPolicyOption configures an <a href="#InvocationRetryPolicy">InvocationRetryPolicy</a>.
</p>

  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="MockableContext" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/context.go#L71">MockableContext</a> <a class="Documentation-idLink" href="#MockableContext" title="Go to MockableContext" aria-label="Go to MockableContext">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.25.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type MockableContext = <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/restatecontext">restatecontext</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/restatecontext#Context">Context</a></pre>
    </div>
  <p>MockableContext is the context interface that test mocks implement. To be used with
*MockContext from the github.com/restatedev/sdk-go/x/mocks module.
</p>

  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="ObjectContext" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/context.go#L31">ObjectContext</a> <a class="Documentation-idLink" href="#ObjectContext" title="Go to ObjectContext" aria-label="Go to ObjectContext">¶</a></span>
  <span class="Documentation-sinceVersion">
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type ObjectContext interface {
	<a href="#ObjectSharedContext">ObjectSharedContext</a>
	<span class="comment">// contains filtered or unexported methods</span>
}</pre>
    </div>
  <p>ObjectContext is an extension of <a href="#ObjectSharedContext">ObjectSharedContext</a> which is passed to exclusive-mode Virtual Object handlers.
giving mutable access to state.
</p>

  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="ObjectHandlerFn" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/handler.go#L27">ObjectHandlerFn</a> <a class="Documentation-idLink" href="#ObjectHandlerFn" title="Go to ObjectHandlerFn" aria-label="Go to ObjectHandlerFn">¶</a></span>
  <span class="Documentation-sinceVersion">
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type ObjectHandlerFn[I <a href="/builtin#any">any</a>, O <a href="/builtin#any">any</a>] func(ctx <a href="#ObjectContext">ObjectContext</a>, input I) (O, <a href="/builtin#error">error</a>)</pre>
    </div>
  <p>ObjectHandlerFn is the signature for a Virtual Object exclusive handler function
</p>

  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="ObjectSharedContext" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/context.go#L24">ObjectSharedContext</a> <a class="Documentation-idLink" href="#ObjectSharedContext" title="Go to ObjectSharedContext" aria-label="Go to ObjectSharedContext">¶</a></span>
  <span class="Documentation-sinceVersion">
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type ObjectSharedContext interface {
	<a href="#Context">Context</a>
	<span class="comment">// contains filtered or unexported methods</span>
}</pre>
    </div>
  <p>ObjectSharedContext is an extension of <a href="#Context">Context</a> which is passed to shared-mode Virtual Object handlers,
giving read-only access to a snapshot of state.
</p>

  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="ObjectSharedHandlerFn" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/handler.go#L30">ObjectSharedHandlerFn</a> <a class="Documentation-idLink" href="#ObjectSharedHandlerFn" title="Go to ObjectSharedHandlerFn" aria-label="Go to ObjectSharedHandlerFn">¶</a></span>
  <span class="Documentation-sinceVersion">
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type ObjectSharedHandlerFn[I <a href="/builtin#any">any</a>, O <a href="/builtin#any">any</a>] func(ctx <a href="#ObjectSharedContext">ObjectSharedContext</a>, input I) (O, <a href="/builtin#error">error</a>)</pre>
    </div>
  <p>ObjectSharedHandlerFn is the signature for a Virtual Object shared-mode handler function
</p>

  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="OnMaxAttempts" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/invocation_options.go#L17">OnMaxAttempts</a> <a class="Documentation-idLink" href="#OnMaxAttempts" title="Go to OnMaxAttempts" aria-label="Go to OnMaxAttempts">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.20.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type OnMaxAttempts = <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#OnMaxAttempts">OnMaxAttempts</a></pre>
    </div>
  

  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="PromiseOption" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/promises.go#L9">PromiseOption</a> <a class="Documentation-idLink" href="#PromiseOption" title="Go to PromiseOption" aria-label="Go to PromiseOption">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.19.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type PromiseOption = <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#PromiseOption">PromiseOption</a></pre>
    </div>
  <p>PromiseOption is an option for <a href="#Promise">Promise</a>.
</p>

  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="Request" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/context.go#L13">Request</a> <a class="Documentation-idLink" href="#Request" title="Go to Request" aria-label="Go to Request">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.9.1</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type Request = <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/restatecontext">restatecontext</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/restatecontext#Request">Request</a></pre>
    </div>
  <p>Request contains a set of information about the request that started an invocation
</p>

  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="RequestOption" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/rpc.go#L16">RequestOption</a> <a class="Documentation-idLink" href="#RequestOption" title="Go to RequestOption" aria-label="Go to RequestOption">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.19.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type RequestOption = <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#RequestOption">RequestOption</a></pre>
    </div>
  <p>RequestOption is an option for a <a href="#Client.Request">Client.Request</a> or <a href="#Client.RequestFuture">Client.RequestFuture</a> call.
</p>

  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="ResolveAwakeableOption" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/awakeables.go#L13">ResolveAwakeableOption</a> <a class="Documentation-idLink" href="#ResolveAwakeableOption" title="Go to ResolveAwakeableOption" aria-label="Go to ResolveAwakeableOption">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.19.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type ResolveAwakeableOption = <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#ResolveAwakeableOption">ResolveAwakeableOption</a></pre>
    </div>
  <p>ResolveAwakeableOption is an option for <a href="#ResolveAwakeable">ResolveAwakeable</a>.
</p>

  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="ResolveSignalOption" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/signals.go#L13">ResolveSignalOption</a> <a class="Documentation-idLink" href="#ResolveSignalOption" title="Go to ResolveSignalOption" aria-label="Go to ResolveSignalOption">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.25.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type ResolveSignalOption = <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#ResolveSignalOption">ResolveSignalOption</a></pre>
    </div>
  <p>ResolveSignalOption is an option for <a href="#ResolveSignal">ResolveSignal</a>.
</p>

  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="ResponseFuture" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/rpc.go#L111">ResponseFuture</a> <a class="Documentation-idLink" href="#ResponseFuture" title="Go to ResponseFuture" aria-label="Go to ResponseFuture">¶</a></span>
  <span class="Documentation-sinceVersion">
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type ResponseFuture[O <a href="/builtin#any">any</a>] interface {
<span id="ResponseFuture.Response" data-kind="method">	<span class="comment">// Response blocks on the response to the call and returns it or the associated error</span>
</span>	<span class="comment">// It is *not* safe to call this in a goroutine - use Context.Select if you</span>
	<span class="comment">// want to wait on multiple results at once.</span>
	Response() (O, <a href="#TerminalError">TerminalError</a>)
	<a href="#Invocation">Invocation</a>
	<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/restatecontext">restatecontext</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/restatecontext#Future">Future</a>
}</pre>
    </div>
  <p>ResponseFuture is a handle on a potentially not-yet completed outbound call.
</p>

  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="RetryableError" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/error.go#L74">RetryableError</a> <a class="Documentation-idLink" href="#RetryableError" title="Go to RetryableError" aria-label="Go to RetryableError">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.25.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type RetryableError = <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/errors">errors</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/errors#RetryableError">RetryableError</a></pre>
    </div>
  <p>RetryableError finishes an attempt with a non-terminal failure: the invocation (or a
Run closure) is retried rather than completed. It carries a <a href="#Code">Code</a> and a message,
wraps the underlying error, and implements the error interface. Returning one from a
handler or Run closure is equivalent to returning any non-terminal error - Restate
retries - except that its code is carried through. Use <a href="#RetryableErrorf">RetryableErrorf</a> or
<a href="#ToRetryableError">ToRetryableError</a> to construct one.
</p>
<div class="Documentation-typeFunc">
    
  
  
    <h4 tabindex="-1" id="AsRetryableError" data-kind="function" class="Documentation-typeFuncHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/error.go#L101">AsRetryableError</a> <a class="Documentation-idLink" href="#AsRetryableError" title="Go to AsRetryableError" aria-label="Go to AsRetryableError">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.25.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func AsRetryableError(err <a href="/builtin#error">error</a>) <a href="#RetryableError">RetryableError</a></pre>
    </div>
  <p>AsRetryableError casts the current error to <a href="#RetryableError">RetryableError</a> if any.
</p>

  

  </div><div class="Documentation-typeFunc">
    
  
  
    <h4 tabindex="-1" id="RetryableErrorf" data-kind="function" class="Documentation-typeFuncHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/error.go#L91">RetryableErrorf</a> <a class="Documentation-idLink" href="#RetryableErrorf" title="Go to RetryableErrorf" aria-label="Go to RetryableErrorf">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.25.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func RetryableErrorf(format <a href="/builtin#string">string</a>, a ...<a href="/builtin#any">any</a>) <a href="#RetryableError">RetryableError</a></pre>
    </div>
  <p>RetryableErrorf builds a <a href="#RetryableError">RetryableError</a> whose message is fmt.Sprintf(format, a...).
To attach a code, build the message with fmt.Errorf and pass it to <a href="#ToRetryableError">ToRetryableError</a>
with <a href="#WithErrorCode">WithErrorCode</a>.
</p>

  

  </div><div class="Documentation-typeFunc">
    
  
  
    <h4 tabindex="-1" id="ToRetryableError" data-kind="function" class="Documentation-typeFuncHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/error.go#L84">ToRetryableError</a> <a class="Documentation-idLink" href="#ToRetryableError" title="Go to ToRetryableError" aria-label="Go to ToRetryableError">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.25.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func ToRetryableError(err <a href="/builtin#error">error</a>, opts ...<a href="#RetryableErrorOption">RetryableErrorOption</a>) <a href="#RetryableError">RetryableError</a></pre>
    </div>
  <p>ToRetryableError converts err into a <a href="#RetryableError">RetryableError</a>. It returns nil if err is nil;
if err already is, or wraps, a <a href="#RetryableError">RetryableError</a> and no options are given, that
<a href="#RetryableError">RetryableError</a> is returned unchanged; otherwise err is wrapped (errors.Unwrap,
errors.Is and errors.As reach err through the result). The code defaults to 500 unless
set with <a href="#WithErrorCode">WithErrorCode</a>.
</p>

  

  </div>
  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="RetryableErrorOption" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/error.go#L77">RetryableErrorOption</a> <a class="Documentation-idLink" href="#RetryableErrorOption" title="Go to RetryableErrorOption" aria-label="Go to RetryableErrorOption">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.25.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type RetryableErrorOption = <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/errors">errors</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/errors#RetryableErrorOption">RetryableErrorOption</a></pre>
    </div>
  <p>RetryableErrorOption customizes a <a href="#RetryableError">RetryableError</a>. Pass it to <a href="#ToRetryableError">ToRetryableError</a>.
</p>

  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="RunAsyncFuture" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/run.go#L95">RunAsyncFuture</a> <a class="Documentation-idLink" href="#RunAsyncFuture" title="Go to RunAsyncFuture" aria-label="Go to RunAsyncFuture">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.17.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type RunAsyncFuture[T <a href="/builtin#any">any</a>] interface {
<span id="RunAsyncFuture.Result" data-kind="method">	<span class="comment">// Result blocks on receiving the RunAsync result, returning the value it was</span>
</span>	<span class="comment">// resolved or otherwise returning the error it was rejected with.</span>
	<span class="comment">// It is *not* safe to call this in a goroutine - use Context.Select if you</span>
	<span class="comment">// want to wait on multiple results at once.</span>
	Result() (T, <a href="#TerminalError">TerminalError</a>)
	<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/restatecontext">restatecontext</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/restatecontext#Future">Future</a>
}</pre>
    </div>
  <p>RunAsyncFuture is a &#39;promise&#39; for a RunAsync operation.
</p>
<div class="Documentation-typeFunc">
    
  
  
    <h4 tabindex="-1" id="RunAsync" data-kind="function" class="Documentation-typeFuncHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/run.go#L69">RunAsync</a> <a class="Documentation-idLink" href="#RunAsync" title="Go to RunAsync" aria-label="Go to RunAsync">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.17.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func RunAsync[T <a href="/builtin#any">any</a>](ctx <a href="#Context">Context</a>, fn func(ctx <a href="#RunContext">RunContext</a>) (T, <a href="/builtin#error">error</a>), options ...<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#RunOption">RunOption</a>) <a href="#RunAsyncFuture">RunAsyncFuture</a>[T]</pre>
    </div>
  <p>RunAsync runs the function (fn), storing final results (including terminal errors)
durably in the journal, or otherwise for transient errors stopping execution
so Restate can retry the invocation. Replays will produce the same value, so
all non-deterministic operations (eg, generating a unique ID) *must* happen
inside Run blocks.
</p><p>This is similar to Run, but it returns a RunAsyncFuture instead that can be used within a WaitFirst, Wait.
</p><p>IMPORTANT: Only use the RunContext parameter provided to the function, NOT the
handler&#39;s Context. See the Run function documentation for detailed examples and guidelines.
</p>

  

  </div>
  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="RunContext" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/context.go#L10">RunContext</a> <a class="Documentation-idLink" href="#RunContext" title="Go to RunContext" aria-label="Go to RunContext">¶</a></span>
  <span class="Documentation-sinceVersion">
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type RunContext = <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/restatecontext">restatecontext</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/restatecontext#RunContext">RunContext</a></pre>
    </div>
  <p>RunContext is passed to <a href="#Run">Run</a> closures and provides the limited set of Restate operations that are safe to use there.
</p>

  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="RunOption" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/run.go#L10">RunOption</a> <a class="Documentation-idLink" href="#RunOption" title="Go to RunOption" aria-label="Go to RunOption">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.19.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type RunOption = <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#RunOption">RunOption</a></pre>
    </div>
  <p>RunOption is an option for <a href="#Run">Run</a>, <a href="#RunAsync">RunAsync</a> and <a href="#RunVoid">RunVoid</a>.
</p>

  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="SendClient" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/rpc.go#L64">SendClient</a> <a class="Documentation-idLink" href="#SendClient" title="Go to SendClient" aria-label="Go to SendClient">¶</a></span>
  <span class="Documentation-sinceVersion">
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type SendClient[I <a href="/builtin#any">any</a>] interface {
<span id="SendClient.Send" data-kind="method">	<span class="comment">// Send makes a one-way call which is executed in the background</span>
</span>	Send(input I, options ...<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#SendOption">SendOption</a>) <a href="#Invocation">Invocation</a>
}</pre>
    </div>
  <p>SendClient allows making one-way invocations
</p>
<div class="Documentation-typeFunc">
    
  
  
    <h4 tabindex="-1" id="ObjectSend" data-kind="function" class="Documentation-typeFuncHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/rpc.go#L40">ObjectSend</a> <a class="Documentation-idLink" href="#ObjectSend" title="Go to ObjectSend" aria-label="Go to ObjectSend">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.11.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func ObjectSend(ctx <a href="#Context">Context</a>, service <a href="/builtin#string">string</a>, key <a href="/builtin#string">string</a>, method <a href="/builtin#string">string</a>, options ...<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#ClientOption">ClientOption</a>) <a href="#SendClient">SendClient</a>[<a href="/builtin#any">any</a>]</pre>
    </div>
  <p>ObjectSend gets an Object send client by service name, key and method name
</p>

  

  </div><div class="Documentation-typeFunc">
    
  
  
    <h4 tabindex="-1" id="ServiceSend" data-kind="function" class="Documentation-typeFuncHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/rpc.go#L30">ServiceSend</a> <a class="Documentation-idLink" href="#ServiceSend" title="Go to ServiceSend" aria-label="Go to ServiceSend">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.11.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func ServiceSend(ctx <a href="#Context">Context</a>, service <a href="/builtin#string">string</a>, method <a href="/builtin#string">string</a>, options ...<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#ClientOption">ClientOption</a>) <a href="#SendClient">SendClient</a>[<a href="/builtin#any">any</a>]</pre>
    </div>
  <p>ServiceSend gets a Service send client by service and method name
</p>

  

  </div><div class="Documentation-typeFunc">
    
  
  
    <h4 tabindex="-1" id="WorkflowSend" data-kind="function" class="Documentation-typeFuncHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/rpc.go#L50">WorkflowSend</a> <a class="Documentation-idLink" href="#WorkflowSend" title="Go to WorkflowSend" aria-label="Go to WorkflowSend">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.12.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func WorkflowSend(ctx <a href="#Context">Context</a>, service <a href="/builtin#string">string</a>, workflowID <a href="/builtin#string">string</a>, method <a href="/builtin#string">string</a>, options ...<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#ClientOption">ClientOption</a>) <a href="#SendClient">SendClient</a>[<a href="/builtin#any">any</a>]</pre>
    </div>
  <p>WorkflowSend gets a Workflow send client by service name, workflow ID and method name
</p>

  

  </div>
  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="SendOption" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/rpc.go#L19">SendOption</a> <a class="Documentation-idLink" href="#SendOption" title="Go to SendOption" aria-label="Go to SendOption">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.19.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type SendOption = <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#SendOption">SendOption</a></pre>
    </div>
  <p>SendOption is an option for a <a href="#SendClient.Send">SendClient.Send</a> call.
</p>

  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="ServiceDefinition" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/service_definition.go#L11">ServiceDefinition</a> <a class="Documentation-idLink" href="#ServiceDefinition" title="Go to ServiceDefinition" aria-label="Go to ServiceDefinition">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.10.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type ServiceDefinition interface {
<span id="ServiceDefinition.Name" data-kind="method">	<span class="comment">// Name returns the name of the service described in this definition</span>
</span>	Name() <a href="/builtin#string">string</a>
<span id="ServiceDefinition.Type" data-kind="method">	<span class="comment">// Type returns the type of this service definition (Service or Virtual Object)</span>
</span>	Type() <a href="/github.com/restatedev/sdk-go@v1.1.0/internal">internal</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal#ServiceType">ServiceType</a>
<span id="ServiceDefinition.Handlers" data-kind="method">	<span class="comment">// Handlers returns the set of handlers associated with this service definition</span>
</span>	Handlers() map[<a href="/builtin#string">string</a>]<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/restatecontext">restatecontext</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/restatecontext#Handler">Handler</a>
<span id="ServiceDefinition.GetOptions" data-kind="method">	<span class="comment">// GetOptions returns the configured options</span>
</span>	GetOptions() *<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#ServiceDefinitionOptions">ServiceDefinitionOptions</a>
<span id="ServiceDefinition.ConfigureHandler" data-kind="method">	<span class="comment">// ConfigureHandler lets you customize the handler configuration, adding per handler options.</span>
</span>	<span class="comment">// Panics if the handler doesn&#39;t exist.</span>
	ConfigureHandler(name <a href="/builtin#string">string</a>, opts ...<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#HandlerOption">HandlerOption</a>) <a href="#ServiceDefinition">ServiceDefinition</a>
}</pre>
    </div>
  <p>ServiceDefinition is the set of methods implemented by both services and virtual objects
</p>
<div class="Documentation-typeFunc">
    
  
  
    <h4 tabindex="-1" id="Reflect" data-kind="function" class="Documentation-typeFuncHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/reflect.go#L60">Reflect</a> <a class="Documentation-idLink" href="#Reflect" title="Go to Reflect" aria-label="Go to Reflect">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.10.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func Reflect(rcvr <a href="/builtin#any">any</a>, opts ...<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#ServiceDefinitionOption">ServiceDefinitionOption</a>) <a href="#ServiceDefinition">ServiceDefinition</a></pre>
    </div>
  <p>Reflect converts a struct with methods into a service definition where each correctly-typed
and exported method of the struct will become a handler in the definition. The service name
defaults to the name of the struct, but this can be overidden by providing a `ServiceName() string` method.
The handler name is the name of the method. Handler methods should have one of the following signatures:
- (ctx, I) (O, error)
- (ctx, I) (O)
- (ctx, I) (error)
- (ctx, I)
- (ctx)
- (ctx) (error)
- (ctx) (O)
- (ctx) (O, error)
Where ctx is <a href="#WorkflowContext">WorkflowContext</a>, <a href="#WorkflowSharedContext">WorkflowSharedContext</a>, <a href="#ObjectContext">ObjectContext</a>, <a href="#ObjectSharedContext">ObjectSharedContext</a> or <a href="#Context">Context</a>. Other signatures are ignored.
Signatures without an I or O type will be treated as if <a href="#Void">Void</a> was provided.
This function will panic if a mixture of object service and workflow method signatures or opts are provided, or if multiple WorkflowContext
methods are defined.
</p><p>Input types will be deserialised with the provided codec (defaults to JSON) except when they are <a href="#Void">Void</a>,
in which case no input bytes or content type may be sent.
Output types will be serialised with the provided codec (defaults to JSON) except when they are <a href="#Void">Void</a>,
in which case no data will be sent and no content type set.
</p>

  

  </div>
  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="ServiceDefinitionOption" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/invocation_options.go#L13">ServiceDefinitionOption</a> <a class="Documentation-idLink" href="#ServiceDefinitionOption" title="Go to ServiceDefinitionOption" aria-label="Go to ServiceDefinitionOption">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.10.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type ServiceDefinitionOption = <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#ServiceDefinitionOption">ServiceDefinitionOption</a></pre>
    </div>
  <p>ServiceDefinitionOption is an option applied to a whole service/object/workflow at registration.
</p>

  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="ServiceHandlerFn" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/handler.go#L24">ServiceHandlerFn</a> <a class="Documentation-idLink" href="#ServiceHandlerFn" title="Go to ServiceHandlerFn" aria-label="Go to ServiceHandlerFn">¶</a></span>
  <span class="Documentation-sinceVersion">
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type ServiceHandlerFn[I <a href="/builtin#any">any</a>, O <a href="/builtin#any">any</a>] func(ctx <a href="#Context">Context</a>, input I) (O, <a href="/builtin#error">error</a>)</pre>
    </div>
  <p>ServiceHandlerFn is the signature for a Service handler function
</p>

  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="SetOption" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/state.go#L11">SetOption</a> <a class="Documentation-idLink" href="#SetOption" title="Go to SetOption" aria-label="Go to SetOption">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.19.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type SetOption = <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#SetOption">SetOption</a></pre>
    </div>
  <p>SetOption is an option for <a href="#Set">Set</a>.
</p>

  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="SignalFuture" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/signals.go#L21">SignalFuture</a> <a class="Documentation-idLink" href="#SignalFuture" title="Go to SignalFuture" aria-label="Go to SignalFuture">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.25.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type SignalFuture[T <a href="/builtin#any">any</a>] interface {
<span id="SignalFuture.Result" data-kind="method">	<span class="comment">// Result blocks on receiving the result of the signal, returning the value it was</span>
</span>	<span class="comment">// resolved with or the error it was rejected with.</span>
	<span class="comment">// It is *not* safe to call this in a goroutine - use Context.Select if you</span>
	<span class="comment">// want to wait on multiple results at once.</span>
	Result() (T, <a href="#TerminalError">TerminalError</a>)
	<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/restatecontext">restatecontext</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/restatecontext#Future">Future</a>
}</pre>
    </div>
  <p>SignalFuture is a promise to a future signal value or error.
</p>
<div class="Documentation-typeFunc">
    
  
  
    <h4 tabindex="-1" id="Signal" data-kind="function" class="Documentation-typeFuncHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/signals.go#L16">Signal</a> <a class="Documentation-idLink" href="#Signal" title="Go to Signal" aria-label="Go to Signal">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.25.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func Signal[T <a href="/builtin#any">any</a>](ctx <a href="#Context">Context</a>, name <a href="/builtin#string">string</a>, options ...<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#SignalOption">SignalOption</a>) <a href="#SignalFuture">SignalFuture</a>[T]</pre>
    </div>
  <p>Signal returns a future for a signal by name.
</p>

  

  </div>
  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="SignalOption" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/signals.go#L10">SignalOption</a> <a class="Documentation-idLink" href="#SignalOption" title="Go to SignalOption" aria-label="Go to SignalOption">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.25.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type SignalOption = <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#SignalOption">SignalOption</a></pre>
    </div>
  <p>SignalOption is an option for <a href="#Signal">Signal</a>.
</p>

  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="SleepOption" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/timers.go#L11">SleepOption</a> <a class="Documentation-idLink" href="#SleepOption" title="Go to SleepOption" aria-label="Go to SleepOption">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.19.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type SleepOption = <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#SleepOption">SleepOption</a></pre>
    </div>
  <p>SleepOption is an option for <a href="#Sleep">Sleep</a> and <a href="#After">After</a>.
</p>

  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="StringMap" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/types.go#L9">StringMap</a> <a class="Documentation-idLink" href="#StringMap" title="Go to StringMap" aria-label="Go to StringMap">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.25.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type StringMap = <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/stringmap">stringmap</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/stringmap#Map">Map</a></pre>
    </div>
  <p>StringMap is a read-only, deterministically-ordered view over string key/value pairs.
</p><p>Iterating a StringMap is deterministic (key-sorted), unlike ranging over a Go map. Use
ToMap when you need a plain map to hand to external code.
</p>

  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="TerminalError" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/error.go#L29">TerminalError</a> <a class="Documentation-idLink" href="#TerminalError" title="Go to TerminalError" aria-label="Go to TerminalError">¶</a></span>
  <span class="Documentation-sinceVersion">
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type TerminalError = <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/errors">errors</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/errors#TerminalError">TerminalError</a></pre>
    </div>
  <p>TerminalError finishes an invocation (or a Run function) with a failure result
instead of being retried. By default, Restate retries the invocation or Run
function forever unless a terminal error is returned.
</p><p>It carries a status code, a message and optional metadata, accessible via the
Code, Message and Metadata methods, and implements the error interface. Use
<a href="#TerminalErrorf">TerminalErrorf</a> or <a href="#ToTerminalError">ToTerminalError</a> to construct one.
</p>
<div class="Documentation-typeFunc">
    
  
  
    <h4 tabindex="-1" id="AsTerminalError" data-kind="function" class="Documentation-typeFuncHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/error.go#L64">AsTerminalError</a> <a class="Documentation-idLink" href="#AsTerminalError" title="Go to AsTerminalError" aria-label="Go to AsTerminalError">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.25.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func AsTerminalError(err <a href="/builtin#error">error</a>) <a href="#TerminalError">TerminalError</a></pre>
    </div>
  <p>AsTerminalError casts the current error to <a href="#TerminalError">TerminalError</a> if any.
</p>

  

  </div><div class="Documentation-typeFunc">
    
  
  
    <h4 tabindex="-1" id="Get" data-kind="function" class="Documentation-typeFuncHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/state.go#L17">Get</a> <a class="Documentation-idLink" href="#Get" title="Go to Get" aria-label="Go to Get">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.11.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func Get[T <a href="/builtin#any">any</a>](ctx <a href="#ObjectSharedContext">ObjectSharedContext</a>, key <a href="/builtin#string">string</a>, options ...<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#GetOption">GetOption</a>) (output T, err <a href="#TerminalError">TerminalError</a>)</pre>
    </div>
  <p>Get gets the value for a key. If there is no associated value with key, the zero value is returned.
To check explicitly for this case pass a pointer eg *string as T.
If the invocation was cancelled while obtaining the state (only possible if eager state is disabled),
a cancellation error is returned.
</p>

  

  </div><div class="Documentation-typeFunc">
    
  
  
    <h4 tabindex="-1" id="Keys" data-kind="function" class="Documentation-typeFuncHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/state.go#L23">Keys</a> <a class="Documentation-idLink" href="#Keys" title="Go to Keys" aria-label="Go to Keys">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.11.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func Keys(ctx <a href="#ObjectSharedContext">ObjectSharedContext</a>) ([]<a href="/builtin#string">string</a>, <a href="#TerminalError">TerminalError</a>)</pre>
    </div>
  <p>Keys retrieves all the state keys set inside a virtual object instance.
</p>

  

  </div><div class="Documentation-typeFunc">
    
  
  
    <h4 tabindex="-1" id="Run" data-kind="function" class="Documentation-typeFuncHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/run.go#L51">Run</a> <a class="Documentation-idLink" href="#Run" title="Go to Run" aria-label="Go to Run">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.11.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func Run[T <a href="/builtin#any">any</a>](ctx <a href="#Context">Context</a>, fn func(ctx <a href="#RunContext">RunContext</a>) (T, <a href="/builtin#error">error</a>), options ...<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#RunOption">RunOption</a>) (output T, err <a href="#TerminalError">TerminalError</a>)</pre>
    </div>
  <p>Run runs the function (fn), storing final results (including terminal errors)
durably in the journal, or otherwise for transient errors stopping execution
so Restate can retry the invocation. Replays will produce the same value, so
all non-deterministic operations (eg, generating a unique ID) *must* happen
inside Run blocks.
</p><p>Inside Run blocks, you can only:
</p><ul class="Documentation-bulletList">
  <li>Perform non-deterministic operations (random number generation, external API calls, etc.)</li>
  <li>Use standard Go operations (math, string manipulation, etc.)</li>
</ul><p>You CANNOT use inside Run blocks:
</p><ul class="Documentation-bulletList">
  <li>Any Restate SDK operations that require the handler Context</li>
</ul><p>See: <a href="https://docs.restate.dev/develop/go/durable-steps">https://docs.restate.dev/develop/go/durable-steps</a>
</p><p>IMPORTANT: Only use the RunContext parameter provided to the function, NOT the
handler&#39;s Context. The RunContext parameter intentionally shadows the handler
context to prevent accidental misuse. Using the handler context inside Run leads
to concurrency issues and undefined behavior.
</p><p>Example:
</p><pre>func (s *Service) MyHandler(ctx restate.Context, input string) (string, error) {
	result, err := restate.Run(ctx, func(ctx restate.RunContext) (string, error) {
		// Use the RunContext parameter &#39;ctx&#39; here - it shadows the handler context
		return doNonDeterministicOperation(ctx)
	})
	return result, err
}
</pre><p>Example (INCORRECT - DO NOT DO THIS):
</p><pre>func (s *Service) MyHandler(ctx restate.Context, input string) (string, error) {
	result, err := restate.Run(ctx, func(runCtx restate.RunContext) (string, error) {
		// WRONG: Using handler context &#39;ctx&#39; instead of &#39;runCtx&#39;
		return doNonDeterministicOperation(ctx)  // This will cause concurrency issues!
	})
	return result, err
}
</pre>

  

  </div><div class="Documentation-typeFunc">
    
  
  
    <h4 tabindex="-1" id="RunVoid" data-kind="function" class="Documentation-typeFuncHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/run.go#L85">RunVoid</a> <a class="Documentation-idLink" href="#RunVoid" title="Go to RunVoid" aria-label="Go to RunVoid">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.22.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func RunVoid(ctx <a href="#Context">Context</a>, fn func(ctx <a href="#RunContext">RunContext</a>) <a href="/builtin#error">error</a>, options ...<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#RunOption">RunOption</a>) <a href="#TerminalError">TerminalError</a></pre>
    </div>
  <p>RunVoid runs the function (fn), storing final results (including terminal errors)
durably in the journal, or otherwise for transient errors stopping execution
so Restate can retry the invocation. Replays will produce the same value, so
all non-deterministic operations (eg, generating a unique ID) *must* happen
inside RunVoid blocks.
</p><p>This is similar to Run, but for functions that don&#39;t return a value.
</p><p>IMPORTANT: Only use the RunContext parameter provided to the function, NOT the
handler&#39;s Context. See the Run function documentation for detailed examples and guidelines.
</p>

  

  </div><div class="Documentation-typeFunc">
    
  
  
    <h4 tabindex="-1" id="Sleep" data-kind="function" class="Documentation-typeFuncHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/timers.go#L14">Sleep</a> <a class="Documentation-idLink" href="#Sleep" title="Go to Sleep" aria-label="Go to Sleep">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.11.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func Sleep(ctx <a href="#Context">Context</a>, d <a href="/time">time</a>.<a href="/time#Duration">Duration</a>, opts ...<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options#SleepOption">SleepOption</a>) <a href="#TerminalError">TerminalError</a></pre>
    </div>
  <p>Sleep for the duration d. Can return a terminal error in the case where the invocation was cancelled mid-sleep.
</p>

  

  </div><div class="Documentation-typeFunc">
    
  
  
    <h4 tabindex="-1" id="TerminalErrorf" data-kind="function" class="Documentation-typeFuncHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/error.go#L52">TerminalErrorf</a> <a class="Documentation-idLink" href="#TerminalErrorf" title="Go to TerminalErrorf" aria-label="Go to TerminalErrorf">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.11.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func TerminalErrorf(format <a href="/builtin#string">string</a>, a ...<a href="/builtin#any">any</a>) <a href="#TerminalError">TerminalError</a></pre>
    </div>
  <p>TerminalErrorf builds a <a href="#TerminalError">TerminalError</a> whose message is fmt.Sprintf(format, a...).
To attach a code or metadata, build the message with fmt.Errorf and pass it to
<a href="#ToTerminalError">ToTerminalError</a> with the relevant options.
</p>

  

  </div><div class="Documentation-typeFunc">
    
  
  
    <h4 tabindex="-1" id="ToTerminalError" data-kind="function" class="Documentation-typeFuncHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/error.go#L45">ToTerminalError</a> <a class="Documentation-idLink" href="#ToTerminalError" title="Go to ToTerminalError" aria-label="Go to ToTerminalError">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.25.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func ToTerminalError(err <a href="/builtin#error">error</a>, opts ...<a href="#TerminalErrorOption">TerminalErrorOption</a>) <a href="#TerminalError">TerminalError</a></pre>
    </div>
  <p>ToTerminalError converts err into a <a href="#TerminalError">TerminalError</a>, so that returning it from a
handler or Run finishes the invocation with a failure result instead of being
retried.
</p><p>IMPORTANT: this does NOT wrap err. A <a href="#TerminalError">TerminalError</a> carries no nested error and is
not part of err&#39;s chain: errors.Unwrap, errors.Is and errors.As will not reach err
through the result. Only the message, err.Error(), is copied.
</p><p>It returns nil if err is nil; if err already is, or wraps, a <a href="#TerminalError">TerminalError</a> and no
options are given, that <a href="#TerminalError">TerminalError</a> is returned unchanged. The code defaults to
500 unless set with <a href="#WithErrorCode">WithErrorCode</a>; metadata can be attached with <a href="#WithMetadata">WithMetadata</a>.
</p>

  

  </div>
  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="TerminalErrorOption" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/error.go#L32">TerminalErrorOption</a> <a class="Documentation-idLink" href="#TerminalErrorOption" title="Go to TerminalErrorOption" aria-label="Go to TerminalErrorOption">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.25.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type TerminalErrorOption = <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/errors">errors</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/errors#TerminalErrorOption">TerminalErrorOption</a></pre>
    </div>
  <p>TerminalErrorOption customizes a <a href="#TerminalError">TerminalError</a>. Pass it to <a href="#ToTerminalError">ToTerminalError</a>.
</p>

  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="Void" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/handler.go#L21">Void</a> <a class="Documentation-idLink" href="#Void" title="Go to Void" aria-label="Go to Void">¶</a></span>
  <span class="Documentation-sinceVersion">
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type Void = <a href="/github.com/restatedev/sdk-go@v1.1.0/encoding">encoding</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/encoding#Void">Void</a></pre>
    </div>
  <p>Void is a placeholder to signify &#39;no value&#39; where a type is otherwise needed. It can be used in several contexts:
</p><ol class="Documentation-numberList">
  <li value="1">Input types for handlers - the request payload codec will reject input at the ingress</li>
  <li value="2">Output types for handlers - the response payload codec will send no bytes and set no content-type</li>
  <li value="3">Input for a outgoing Request or Send - no bytes will be sent</li>
  <li value="4">The output type for an outgoing Request - the response body will be ignored. A pointer is also accepted.</li>
  <li value="5">The output type for an awakeable - the result body will be ignored. A pointer is also accepted.</li>
</ol>

  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="WaitIterator" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/futures.go#L118">WaitIterator</a> <a class="Documentation-idLink" href="#WaitIterator" title="Go to WaitIterator" aria-label="Go to WaitIterator">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.21.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type WaitIterator = <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/restatecontext">restatecontext</a>.<a href="/github.com/restatedev/sdk-go@v1.1.0/internal/restatecontext#WaitIterator">WaitIterator</a></pre>
    </div>
  <p>WaitIterator is an iterator over a list of blocking Restate operations that are running
in the background. See WaitIter for more details.
</p>
<div class="Documentation-typeFunc">
    
  
  
    <h4 tabindex="-1" id="WaitIter" data-kind="function" class="Documentation-typeFuncHeader">
      <span>func <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/futures.go#L112">WaitIter</a> <a class="Documentation-idLink" href="#WaitIter" title="Go to WaitIter" aria-label="Go to WaitIter">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.21.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>func WaitIter(ctx <a href="#Context">Context</a>, futs ...<a href="#Future">Future</a>) <a href="#WaitIterator">WaitIterator</a></pre>
    </div>
  <p>WaitIter returns an iterator that allows manual control over waiting for multiple Futures to complete.
This is the low-level primitive that WaitFirst and Wait are built on top of.
Call Next() to wait for the next Future to complete, then use Value() to retrieve it and Err() to check for errors.
</p><p>Example:
</p><pre>func MyHandler(ctx restate.Context, input string) (string, error) {
	fut1 := restate.Service[string](ctx, &#34;service1&#34;, &#34;method1&#34;).RequestFuture(input)
	fut2 := restate.Service[string](ctx, &#34;service2&#34;, &#34;method2&#34;).RequestFuture(input)

	iter := restate.WaitIter(ctx, fut1, fut2)
	for iter.Next() {
		fut := iter.Value()
		// Process each future as it completes
		if fut == fut1 {
			result, _ := fut1.Response()
			fmt.Printf(&#34;fut1 completed with: %s\n&#34;, result)
		}
	}
	if err := iter.Err(); err != nil {
		return &#34;&#34;, err
	}
	return &#34;all done&#34;, nil
}
</pre>

  

  </div>
  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="WorkflowContext" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/context.go#L45">WorkflowContext</a> <a class="Documentation-idLink" href="#WorkflowContext" title="Go to WorkflowContext" aria-label="Go to WorkflowContext">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.12.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type WorkflowContext interface {
	<a href="#WorkflowSharedContext">WorkflowSharedContext</a>
	<a href="#ObjectContext">ObjectContext</a>
	<span class="comment">// contains filtered or unexported methods</span>
}</pre>
    </div>
  <p>WorkflowContext is an extension of <a href="#WorkflowSharedContext">WorkflowSharedContext</a> and <a href="#ObjectContext">ObjectContext</a> which is passed to Workflow &#39;run&#39; handlers,
giving mutable access to state.
</p>

  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="WorkflowHandlerFn" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/handler.go#L33">WorkflowHandlerFn</a> <a class="Documentation-idLink" href="#WorkflowHandlerFn" title="Go to WorkflowHandlerFn" aria-label="Go to WorkflowHandlerFn">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.12.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type WorkflowHandlerFn[I <a href="/builtin#any">any</a>, O <a href="/builtin#any">any</a>] func(ctx <a href="#WorkflowContext">WorkflowContext</a>, input I) (O, <a href="/builtin#error">error</a>)</pre>
    </div>
  <p>ObjectHandlerFn is the signature for a Workflow &#39;Run&#39; handler function
</p>

  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="WorkflowSharedContext" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/context.go#L38">WorkflowSharedContext</a> <a class="Documentation-idLink" href="#WorkflowSharedContext" title="Go to WorkflowSharedContext" aria-label="Go to WorkflowSharedContext">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.12.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type WorkflowSharedContext interface {
	<a href="#ObjectSharedContext">ObjectSharedContext</a>
	<span class="comment">// contains filtered or unexported methods</span>
}</pre>
    </div>
  <p>WorkflowSharedContext is an extension of <a href="#ObjectSharedContext">ObjectSharedContext</a> which is passed to shared-mode Workflow handlers,
giving read-only access to a snapshot of state.
</p>

  

    </div><div class="Documentation-type">
      
  
  
    <h4 tabindex="-1" id="WorkflowSharedHandlerFn" data-kind="type" class="Documentation-typeHeader">
      <span>type <a class="Documentation-source" href="https://github.com/restatedev/sdk-go/blob/v1.1.0/handler.go#L36">WorkflowSharedHandlerFn</a> <a class="Documentation-idLink" href="#WorkflowSharedHandlerFn" title="Go to WorkflowSharedHandlerFn" aria-label="Go to WorkflowSharedHandlerFn">¶</a></span>
  <span class="Documentation-sinceVersion">
    
      <span class="Documentation-sinceVersionLabel">added in</span>
      <span class="Documentation-sinceVersionVersion">v0.12.0</span>
    
  </span>
</h4>

    
    <div class="Documentation-declaration">
      <pre>type WorkflowSharedHandlerFn[I <a href="/builtin#any">any</a>, O <a href="/builtin#any">any</a>] func(ctx <a href="#WorkflowSharedContext">WorkflowSharedContext</a>, input I) (O, <a href="/builtin#error">error</a>)</pre>
    </div>
  <p>WorkflowSharedHandlerFn is the signature for a Workflow shared handler function
</p>

  

    </div></section></div> 







      
    </div>
  </div>

        
      
      
        
  <div class="UnitFiles js-unitFiles">
    <h2 class="UnitFiles-title" id="section-sourcefiles">
      <img class="go-Icon" height="24" width="24" src="/static/shared/icon/insert_drive_file_gm_grey_24dp.svg" alt="">
      Source Files
      <a class="UnitFiles-idLink" href="#section-sourcefiles" title="Go to Source Files" aria-label="Go to Source Files">¶</a>
    </h2><div class="UnitFiles-titleLink">
      <a href="https://github.com/restatedev/sdk-go/tree/v1.1.0" target="_blank" rel="noopener">View all Source files</a>
    </div><div>
      <ul class="UnitFiles-fileList"><li><a href="https://github.com/restatedev/sdk-go/blob/v1.1.0/awakeables.go" target="_blank" rel="noopener" title="awakeables.go">awakeables.go</a></li><li><a href="https://github.com/restatedev/sdk-go/blob/v1.1.0/codec.go" target="_blank" rel="noopener" title="codec.go">codec.go</a></li><li><a href="https://github.com/restatedev/sdk-go/blob/v1.1.0/context.go" target="_blank" rel="noopener" title="context.go">context.go</a></li><li><a href="https://github.com/restatedev/sdk-go/blob/v1.1.0/error.go" target="_blank" rel="noopener" title="error.go">error.go</a></li><li><a href="https://github.com/restatedev/sdk-go/blob/v1.1.0/futures.go" target="_blank" rel="noopener" title="futures.go">futures.go</a></li><li><a href="https://github.com/restatedev/sdk-go/blob/v1.1.0/handler.go" target="_blank" rel="noopener" title="handler.go">handler.go</a></li><li><a href="https://github.com/restatedev/sdk-go/blob/v1.1.0/invocation_options.go" target="_blank" rel="noopener" title="invocation_options.go">invocation_options.go</a></li><li><a href="https://github.com/restatedev/sdk-go/blob/v1.1.0/options.go" target="_blank" rel="noopener" title="options.go">options.go</a></li><li><a href="https://github.com/restatedev/sdk-go/blob/v1.1.0/promises.go" target="_blank" rel="noopener" title="promises.go">promises.go</a></li><li><a href="https://github.com/restatedev/sdk-go/blob/v1.1.0/random.go" target="_blank" rel="noopener" title="random.go">random.go</a></li><li><a href="https://github.com/restatedev/sdk-go/blob/v1.1.0/reflect.go" target="_blank" rel="noopener" title="reflect.go">reflect.go</a></li><li><a href="https://github.com/restatedev/sdk-go/blob/v1.1.0/retry_options.go" target="_blank" rel="noopener" title="retry_options.go">retry_options.go</a></li><li><a href="https://github.com/restatedev/sdk-go/blob/v1.1.0/rpc.go" target="_blank" rel="noopener" title="rpc.go">rpc.go</a></li><li><a href="https://github.com/restatedev/sdk-go/blob/v1.1.0/run.go" target="_blank" rel="noopener" title="run.go">run.go</a></li><li><a href="https://github.com/restatedev/sdk-go/blob/v1.1.0/service_definition.go" target="_blank" rel="noopener" title="service_definition.go">service_definition.go</a></li><li><a href="https://github.com/restatedev/sdk-go/blob/v1.1.0/signals.go" target="_blank" rel="noopener" title="signals.go">signals.go</a></li><li><a href="https://github.com/restatedev/sdk-go/blob/v1.1.0/state.go" target="_blank" rel="noopener" title="state.go">state.go</a></li><li><a href="https://github.com/restatedev/sdk-go/blob/v1.1.0/timers.go" target="_blank" rel="noopener" title="timers.go">timers.go</a></li><li><a href="https://github.com/restatedev/sdk-go/blob/v1.1.0/types.go" target="_blank" rel="noopener" title="types.go">types.go</a></li></ul>
    </div>
  </div>

      
      
        
  <div class="UnitDirectories js-unitDirectories">
    <h2 class="UnitDirectories-title" id="section-directories">
      <img class="go-Icon" height="24" width="24" src="/static/shared/icon/folder_gm_grey_24dp.svg" alt="">
      Directories
      <a class="UnitDirectories-idLink" href="#section-directories" title="Go to Directories" aria-label="Go to Directories">¶</a>
    </h2>
    <div class="UnitDirectories-toggles">
      <div class="UnitDirectories-toggleButtons">
        <button class="js-showInternalDirectories" data-test-id="internal-directories-toggle"
            data-gtmc="directories button" aria-label="Show Internal Directories">
          Show internal
        </button>
        <button class="js-expandAllDirectories" data-test-id="directories-toggle"
            data-gtmc="directories button" aria-label="Expand All Directories">
          Expand all
        </button>
      </div>
    </div>
    <table class="UnitDirectories-table UnitDirectories-table--tree js-expandableTable"
          data-test-id="UnitDirectories-table">
      <tr class="UnitDirectories-tableHeader UnitDirectories-tableHeader--tree">
        <th>Path</th>
        <th class="UnitDirectories-desktopSynopsis">Synopsis</th>
      </tr>
      
          
  
  <tr data-aria-controls="encoding-internal/protojsonschema encoding-internal/protojsonschema/internal/testproto encoding-internal/util "
      class="">
    <td data-id="encoding" data-aria-owns="encoding-internal/protojsonschema encoding-internal/protojsonschema/internal/testproto encoding-internal/util ">
      <div class="UnitDirectories-pathCell">
        <div><button type="button" class="go-Button go-Button--inline UnitDirectories-toggleButton UnitDirectories-internal"
                aria-expanded="false"
                aria-label="3 more from"
                data-aria-controls="encoding-internal/protojsonschema encoding-internal/protojsonschema/internal/testproto encoding-internal/util "
                data-aria-labelledby="encoding-button encoding"
                data-id="encoding-button">
              <img class="go-Icon" height="24" width="24" src="/static/shared/icon/arrow_right_gm_grey_24dp.svg"
                  alt="">
            </button><a href="/github.com/restatedev/sdk-go@v1.1.0/encoding">encoding</a>
            
          </div>
          <div class="UnitDirectories-mobileSynopsis"></div>
        </div>
      </td>
      <td class="UnitDirectories-desktopSynopsis"></td></tr><tr data-id="encoding-internal/protojsonschema" class="UnitDirectories-internal">
      <td>
        <div class="UnitDirectories-subdirectory">
          <span>
            <a href="/github.com/restatedev/sdk-go@v1.1.0/encoding/internal/protojsonschema">internal/protojsonschema</a>
            
            
          </span>
          <div class="UnitDirectories-mobileSynopsis"></div>
        </div>
      </td>
      <td class="UnitDirectories-desktopSynopsis"></td><tr data-id="encoding-internal/protojsonschema/internal/testproto" class="UnitDirectories-internal">
      <td>
        <div class="UnitDirectories-subdirectory">
          <span>
            <a href="/github.com/restatedev/sdk-go@v1.1.0/encoding/internal/protojsonschema/internal/testproto">internal/protojsonschema/internal/testproto</a>
            
            
          </span>
          <div class="UnitDirectories-mobileSynopsis"></div>
        </div>
      </td>
      <td class="UnitDirectories-desktopSynopsis"></td><tr data-id="encoding-internal/util" class="UnitDirectories-internal">
      <td>
        <div class="UnitDirectories-subdirectory">
          <span>
            <a href="/github.com/restatedev/sdk-go@v1.1.0/encoding/internal/util">internal/util</a>
            
            
          </span>
          <div class="UnitDirectories-mobileSynopsis"></div>
        </div>
      </td>
      <td class="UnitDirectories-desktopSynopsis"></td></tr>

      
          
  
  <tr data-aria-controls="examples-codegen examples-otel "
      class="">
    <td data-id="examples" data-aria-owns="examples-codegen examples-otel ">
      <div class="UnitDirectories-pathCell">
        <div><button type="button" class="go-Button go-Button--inline UnitDirectories-toggleButton"
                aria-expanded="false"
                aria-label="2 more from"
                data-aria-controls="examples-codegen examples-otel "
                data-aria-labelledby="examples-button examples"
                data-id="examples-button">
              <img class="go-Icon" height="24" width="24" src="/static/shared/icon/arrow_right_gm_grey_24dp.svg"
                  alt="">
            </button><span>examples</span>
      </td>
      <td class="UnitDirectories-desktopSynopsis"></td></tr><tr data-id="examples-codegen" class="">
      <td>
        <div class="UnitDirectories-subdirectory">
          <span>
            <a href="/github.com/restatedev/sdk-go/examples/codegen">codegen</a>
            <span class="go-Chip go-Chip--inverted">module</span>
            
          </span>
          <div class="UnitDirectories-mobileSynopsis"></div>
        </div>
      </td>
      <td class="UnitDirectories-desktopSynopsis"></td><tr data-id="examples-otel" class="">
      <td>
        <div class="UnitDirectories-subdirectory">
          <span>
            <a href="/github.com/restatedev/sdk-go/examples/otel">otel</a>
            <span class="go-Chip go-Chip--inverted">module</span>
            
          </span>
          <div class="UnitDirectories-mobileSynopsis"></div>
        </div>
      </td>
      <td class="UnitDirectories-desktopSynopsis"></td></tr>

      
          
  
  <tr
      class="">
    <td data-id="ingress" data-aria-owns="">
      <div class="UnitDirectories-pathCell">
        <div><a href="/github.com/restatedev/sdk-go@v1.1.0/ingress">ingress</a>
            
          </div>
          <div class="UnitDirectories-mobileSynopsis"></div>
        </div>
      </td>
      <td class="UnitDirectories-desktopSynopsis"></td></tr></tr>

      
          
  
  <tr data-aria-controls="internal-errors internal-generated internal-genericfutures internal-identity internal-ingress internal-log internal-options internal-randsource internal-restatecontext internal-statemachine internal-stringmap "
      class="UnitDirectories-internal">
    <td data-id="internal" data-aria-owns="internal-errors internal-generated internal-genericfutures internal-identity internal-ingress internal-log internal-options internal-randsource internal-restatecontext internal-statemachine internal-stringmap ">
      <div class="UnitDirectories-pathCell">
        <div><button type="button" class="go-Button go-Button--inline UnitDirectories-toggleButton UnitDirectories-internal"
                aria-expanded="false"
                aria-label="11 more from"
                data-aria-controls="internal-errors internal-generated internal-genericfutures internal-identity internal-ingress internal-log internal-options internal-randsource internal-restatecontext internal-statemachine internal-stringmap "
                data-aria-labelledby="internal-button internal"
                data-id="internal-button">
              <img class="go-Icon" height="24" width="24" src="/static/shared/icon/arrow_right_gm_grey_24dp.svg"
                  alt="">
            </button><a href="/github.com/restatedev/sdk-go@v1.1.0/internal">internal</a>
            
          </div>
          <div class="UnitDirectories-mobileSynopsis"></div>
        </div>
      </td>
      <td class="UnitDirectories-desktopSynopsis"></td></tr><tr data-id="internal-errors" class="UnitDirectories-internal">
      <td>
        <div class="UnitDirectories-subdirectory">
          <span>
            <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/errors">errors</a>
            
            
          </span>
          <div class="UnitDirectories-mobileSynopsis">Package errors holds the SDK&#39;s error model.</div>
        </div>
      </td>
      <td class="UnitDirectories-desktopSynopsis">Package errors holds the SDK&#39;s error model.</td><tr data-id="internal-generated" class="UnitDirectories-internal">
      <td>
        <div class="UnitDirectories-subdirectory">
          <span>
            <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/generated">generated</a>
            
            
          </span>
          <div class="UnitDirectories-mobileSynopsis"></div>
        </div>
      </td>
      <td class="UnitDirectories-desktopSynopsis"></td><tr data-id="internal-genericfutures" class="UnitDirectories-internal">
      <td>
        <div class="UnitDirectories-subdirectory">
          <span>
            <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/genericfutures">genericfutures</a>
            
            
          </span>
          <div class="UnitDirectories-mobileSynopsis"></div>
        </div>
      </td>
      <td class="UnitDirectories-desktopSynopsis"></td><tr data-id="internal-identity" class="UnitDirectories-internal">
      <td>
        <div class="UnitDirectories-subdirectory">
          <span>
            <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/identity">identity</a>
            
            
          </span>
          <div class="UnitDirectories-mobileSynopsis"></div>
        </div>
      </td>
      <td class="UnitDirectories-desktopSynopsis"></td><tr data-id="internal-ingress" class="UnitDirectories-internal">
      <td>
        <div class="UnitDirectories-subdirectory">
          <span>
            <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/ingress">ingress</a>
            
            
          </span>
          <div class="UnitDirectories-mobileSynopsis"></div>
        </div>
      </td>
      <td class="UnitDirectories-desktopSynopsis"></td><tr data-id="internal-log" class="UnitDirectories-internal">
      <td>
        <div class="UnitDirectories-subdirectory">
          <span>
            <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/log">log</a>
            
            
          </span>
          <div class="UnitDirectories-mobileSynopsis"></div>
        </div>
      </td>
      <td class="UnitDirectories-desktopSynopsis"></td><tr data-id="internal-options" class="UnitDirectories-internal">
      <td>
        <div class="UnitDirectories-subdirectory">
          <span>
            <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/options">options</a>
            
            
          </span>
          <div class="UnitDirectories-mobileSynopsis"></div>
        </div>
      </td>
      <td class="UnitDirectories-desktopSynopsis"></td><tr data-id="internal-randsource" class="UnitDirectories-internal">
      <td>
        <div class="UnitDirectories-subdirectory">
          <span>
            <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/randsource">randsource</a>
            
            
          </span>
          <div class="UnitDirectories-mobileSynopsis"></div>
        </div>
      </td>
      <td class="UnitDirectories-desktopSynopsis"></td><tr data-id="internal-restatecontext" class="UnitDirectories-internal">
      <td>
        <div class="UnitDirectories-subdirectory">
          <span>
            <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/restatecontext">restatecontext</a>
            
            
          </span>
          <div class="UnitDirectories-mobileSynopsis"></div>
        </div>
      </td>
      <td class="UnitDirectories-desktopSynopsis"></td><tr data-id="internal-statemachine" class="UnitDirectories-internal">
      <td>
        <div class="UnitDirectories-subdirectory">
          <span>
            <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/statemachine">statemachine</a>
            
            
          </span>
          <div class="UnitDirectories-mobileSynopsis"></div>
        </div>
      </td>
      <td class="UnitDirectories-desktopSynopsis"></td><tr data-id="internal-stringmap" class="UnitDirectories-internal">
      <td>
        <div class="UnitDirectories-subdirectory">
          <span>
            <a href="/github.com/restatedev/sdk-go@v1.1.0/internal/stringmap">stringmap</a>
            
            
          </span>
          <div class="UnitDirectories-mobileSynopsis">Package stringmap provides a read-only, deterministically-ordered view over a set of string key/value pairs.</div>
        </div>
      </td>
      <td class="UnitDirectories-desktopSynopsis">Package stringmap provides a read-only, deterministically-ordered view over a set of string key/value pairs.</td></tr>

      
          
  
  <tr
      class="">
    <td data-id="logging" data-aria-owns="">
      <div class="UnitDirectories-pathCell">
        <div><a href="/github.com/restatedev/sdk-go@v1.1.0/logging">logging</a>
            
          </div>
          <div class="UnitDirectories-mobileSynopsis"></div>
        </div>
      </td>
      <td class="UnitDirectories-desktopSynopsis"></td></tr></tr>

      
          
  
  <tr
      class="">
    <td data-id="server" data-aria-owns="">
      <div class="UnitDirectories-pathCell">
        <div><a href="/github.com/restatedev/sdk-go@v1.1.0/server">server</a>
            
          </div>
          <div class="UnitDirectories-mobileSynopsis"></div>
        </div>
      </td>
      <td class="UnitDirectories-desktopSynopsis"></td></tr></tr>

      
          
  
  <tr
      class="">
    <td data-id="testing" data-aria-owns="">
      <div class="UnitDirectories-pathCell">
        <div><a href="/github.com/restatedev/sdk-go/testing">testing</a>
            <span class="go-Chip go-Chip--inverted">module</span>
          </div>
          <div class="UnitDirectories-mobileSynopsis"></div>
        </div>
      </td>
      <td class="UnitDirectories-desktopSynopsis"></td></tr></tr>

      
          
  
  <tr data-aria-controls="x-mocks x-protoc-gen-go-restate x-tunnel "
      class="">
    <td data-id="x" data-aria-owns="x-mocks x-protoc-gen-go-restate x-tunnel ">
      <div class="UnitDirectories-pathCell">
        <div><button type="button" class="go-Button go-Button--inline UnitDirectories-toggleButton"
                aria-expanded="false"
                aria-label="3 more from"
                data-aria-controls="x-mocks x-protoc-gen-go-restate x-tunnel "
                data-aria-labelledby="x-button x"
                data-id="x-button">
              <img class="go-Icon" height="24" width="24" src="/static/shared/icon/arrow_right_gm_grey_24dp.svg"
                  alt="">
            </button><span>x</span>
      </td>
      <td class="UnitDirectories-desktopSynopsis"></td></tr><tr data-id="x-mocks" class="">
      <td>
        <div class="UnitDirectories-subdirectory">
          <span>
            <a href="/github.com/restatedev/sdk-go/x/mocks">mocks</a>
            <span class="go-Chip go-Chip--inverted">module</span>
            
          </span>
          <div class="UnitDirectories-mobileSynopsis"></div>
        </div>
      </td>
      <td class="UnitDirectories-desktopSynopsis"></td><tr data-id="x-protoc-gen-go-restate" class="">
      <td>
        <div class="UnitDirectories-subdirectory">
          <span>
            <a href="/github.com/restatedev/sdk-go/x/protoc-gen-go-restate">protoc-gen-go-restate</a>
            <span class="go-Chip go-Chip--inverted">module</span>
            
          </span>
          <div class="UnitDirectories-mobileSynopsis"></div>
        </div>
      </td>
      <td class="UnitDirectories-desktopSynopsis"></td><tr data-id="x-tunnel" class="">
      <td>
        <div class="UnitDirectories-subdirectory">
          <span>
            <a href="/github.com/restatedev/sdk-go/x/tunnel">tunnel</a>
            <span class="go-Chip go-Chip--inverted">module</span>
            
          </span>
          <div class="UnitDirectories-mobileSynopsis"></div>
        </div>
      </td>
      <td class="UnitDirectories-desktopSynopsis"></td></tr>

      
    </table>
  </div>

      
    </div>
  </div>
  <div id="showInternal-description" hidden> Click to show internal directories. </div>
  <div id="hideInternal-description" hidden> Click to hide internal directories. </div>
</article>
    <footer class="go-Main-footer"></footer>
  </main>

    
  <footer class="go-Footer">
    
    <div class="go-Footer-links">
      <div class="go-Footer-linkColumn">
        <a href="https://go.dev/solutions" class="go-Footer-link go-Footer-link--primary"
            data-gtmc="footer link">
          Why Go
        </a>
        <a href="https://go.dev/solutions/use-cases" class="go-Footer-link"
            data-gtmc="footer link">
          Use Cases
        </a>
        <a href="https://go.dev/solutions/case-studies" class="go-Footer-link"
            data-gtmc="footer link">
          Case Studies
        </a>
      </div>
      <div class="go-Footer-linkColumn">
        <a href="https://learn.go.dev/" class="go-Footer-link go-Footer-link--primary"
            data-gtmc="footer link">
          Get Started
        </a>
        <a href="https://play.golang.org" class="go-Footer-link" data-gtmc="footer link">
          Playground
        </a>
        <a href="https://tour.golang.org" class="go-Footer-link" data-gtmc="footer link">
          Tour
        </a>
        <a href="https://stackoverflow.com/questions/tagged/go?tab=Newest" class="go-Footer-link"
            data-gtmc="footer link">
          Stack Overflow
        </a>
        <a href="https://go.dev/help" class="go-Footer-link"
            data-gtmc="footer link">
          Help
        </a>
      </div>
      <div class="go-Footer-linkColumn">
        <a href="https://pkg.go.dev" class="go-Footer-link go-Footer-link--primary"
            data-gtmc="footer link">
          Packages
        </a>
        <a href="/std" class="go-Footer-link" data-gtmc="footer link">
          Standard Library
        </a>
        <a href="/golang.org/x" class="go-Footer-link" data-gtmc="footer link">
          Sub-repositories
        </a>
        <a href="https://pkg.go.dev/about" class="go-Footer-link" data-gtmc="footer link">
          About Go Packages
        </a>
         <a href="/api" class="go-Footer-link" data-gtmc="footer link">
          pkg.go.dev API
        </a>
      </div>
      <div class="go-Footer-linkColumn">
        <a href="https://go.dev/project" class="go-Footer-link go-Footer-link--primary"
            data-gtmc="footer link">
          About
        </a>
        <a href="https://go.dev/dl/" class="go-Footer-link" data-gtmc="footer link">Download</a>
        <a href="https://go.dev/blog" class="go-Footer-link" data-gtmc="footer link">Blog</a>
        <a href="https://github.com/golang/go/issues" class="go-Footer-link" data-gtmc="footer link">
          Issue Tracker
        </a>
        <a href="https://go.dev/doc/devel/release.html" class="go-Footer-link"
            data-gtmc="footer link">
          Release Notes
        </a>
        <a href="https://go.dev/brand" class="go-Footer-link" data-gtmc="footer link">
          Brand Guidelines
        </a>
        <a href="https://go.dev/conduct" class="go-Footer-link" data-gtmc="footer link">
          Code of Conduct
        </a>
      </div>
      <div class="go-Footer-linkColumn">
        <a href="https://www.twitter.com/golang" class="go-Footer-link go-Footer-link--primary"
            data-gtmc="footer link">
          Connect
        </a>
        <a href="https://www.twitter.com/golang" class="go-Footer-link" data-gtmc="footer link">
          Twitter
        </a>
        <a href="https://github.com/golang" class="go-Footer-link" data-gtmc="footer link">GitHub</a>
        <a href="https://invite.slack.golangbridge.org/" class="go-Footer-link"
            data-gtmc="footer link">
          Slack
        </a>
        <a href="https://reddit.com/r/golang" class="go-Footer-link" data-gtmc="footer link">
          r/golang
        </a>
        <a href="https://www.meetup.com/pro/go" class="go-Footer-link" data-gtmc="footer link">
          Meetup
        </a>
        <a href="https://golangweekly.com/" class="go-Footer-link" data-gtmc="footer link">
          Golang Weekly
        </a>
      </div>
    </div>
    <div class="go-Footer-bottom">
      <img class="go-Footer-gopher"  width="1431" height="901"
          src="/static/shared/gopher/pilot-bust-1431x901.svg" alt="Gopher in flight goggles">
      <ul class="go-Footer-listRow">
        <li class="go-Footer-listItem">
          <a href="https://go.dev/copyright" data-gtmc="footer link">Copyright</a>
        </li>
        <li class="go-Footer-listItem">
          <a href="https://go.dev/tos" data-gtmc="footer link">Terms of Service</a>
        </li>
        <li class="go-Footer-listItem">
          <a href="http://www.google.com/intl/en/policies/privacy/" data-gtmc="footer link"
              target="_blank" rel="noopener">
            Privacy Policy
          </a>
        </li>
        <li class="go-Footer-listItem">
          <a href="https://go.dev/s/pkgsite-feedback" target="_blank" rel="noopener"
              data-gtmc="footer link">
            Report an Issue
          </a>
        </li>
        <li class="go-Footer-listItem">
          <button class="go-Button go-Button--text go-Footer-toggleTheme js-toggleTheme" aria-label="Theme Toggle">
            <img data-value="auto" class="go-Icon go-Icon--inverted" height="24" width="24" src="/static/shared/icon/brightness_6_gm_grey_24dp.svg" alt="System theme">
            <img data-value="dark" class="go-Icon go-Icon--inverted" height="24" width="24" src="/static/shared/icon/brightness_2_gm_grey_24dp.svg" alt="Dark theme">
            <img data-value="light" class="go-Icon go-Icon--inverted" height="24" width="24" src="/static/shared/icon/light_mode_gm_grey_24dp.svg" alt="Light theme">
            <p> Theme Toggle </p>
          </button>
        </li>
        <li class="go-Footer-listItem">
          <button class="go-Button go-Button--text go-Footer-keyboard js-openShortcuts" aria-label="Shorcuts Modal">
            <img class="go-Icon go-Icon--inverted" height="24" width="24" src="/static/shared/icon/keyboard_grey_24dp.svg" alt="">
            <p> Shortcuts Modal </p>
          </button>
        </li>
      </ul>
      <a class="go-Footer-googleLogo" href="https://google.com" target="_blank"rel="noopener"
          data-gtmc="footer link">
        <img class="go-Footer-googleLogoImg" height="24" width="72"
            src="/static/shared/logo/google-white.svg" alt="Google logo">
      </a>
    </div>
  </footer>

    
  <dialog id="jump-to-modal" class="JumpDialog go-Modal go-Modal--md js-modal">
    <form method="dialog" data-gmtc="jump to form" aria-label="Jump to Identifier">
      <div class="Dialog-title go-Modal-header">
        <h2>Jump to</h2>
        <button
          class="go-Button go-Button--inline"
          type="button"
          data-modal-close
          data-gtmc="modal button"
          aria-label="Close"
        >
          <img
            class="go-Icon"
            height="24"
            width="24"
            src="/static/shared/icon/close_gm_grey_24dp.svg"
            alt=""
          />
        </button>
      </div>
      <div class="JumpDialog-filter">
        <input class="JumpDialog-input go-Input" autocomplete="off" type="text">
      </div>
      <div class="JumpDialog-body go-Modal-body">
        <div class="JumpDialog-list"></div>
      </div>
      <div class="go-Modal-actions">
        <button class="go-Button" data-test-id="close-dialog">Close</button>
      </div>
    </form>
  </dialog>

  <dialog class="ShortcutsDialog go-Modal go-Modal--sm js-modal">
    <form method="dialog">
      <div class="go-Modal-header">
        <h2>Keyboard shortcuts</h2>
        <button
          class="go-Button go-Button--inline"
          type="button"
          data-modal-close
          data-gtmc="modal button"
          aria-label="Close"
        >
          <img
            class="go-Icon"
            height="24"
            width="24"
            src="/static/shared/icon/close_gm_grey_24dp.svg"
            alt=""
          />
        </button>
      </div>
      <div class="go-Modal-body">
        <table>
          <tbody>
            <tr><td class="ShortcutsDialog-key">
              <strong>?</strong></td><td> : This menu</td>
            </tr>
            <tr><td class="ShortcutsDialog-key">
              <strong>/</strong></td><td> : Search site</td>
            </tr>
            <tr><td class="ShortcutsDialog-key">
              <strong>f</strong> or <strong>F</strong></td><td> : Jump to</td>
            </tr>
            <tr>
              <td class="ShortcutsDialog-key"><strong>y</strong> or <strong>Y</strong></td>
              <td> : Canonical URL</td>
            </tr>
          </tbody>
        </table>
      </div>
      <div class="go-Modal-actions">
        <button class="go-Button" data-test-id="close-dialog">Close</button>
      </div>
    </form>
  </dialog>

    
      <section class="Cookie-notice js-cookieNotice">
        <div>go.dev uses cookies from Google to deliver and enhance the quality of its services and to
        analyze traffic. <a target=_blank href="https://policies.google.com/technologies/cookies">Learn more.</a></div>
        <div><button class="go-Button">Okay</button></div>
      </section>
    
    
      <script>
        // this will throw if the querySelector can’t find the element
        const gtmId = document.querySelector('.js-gtmID').dataset.gtmid;
        if (!gtmId) {
          throw new Error('Google Tag Manager ID not found');
        }
        loadScript(`https://www.googletagmanager.com/gtm.js?id=${gtmId}`);
      </script>
      <noscript>
        <iframe src="https://www.googletagmanager.com/ns.html?id=GTM-W8MVQXG"
                height="0" width="0" style="display:none;visibility:hidden">
        </iframe>
      </noscript>
    
    
  
  <div class="js-canonicalURLPath" data-canonical-url-path="/github.com/restatedev/sdk-go@v1.1.0" hidden></div>
  <div class="js-playgroundVars" data-modulepath="github.com/restatedev/sdk-go" data-version="v1.1.0" hidden></div>
  <script>
    loadScript('/static/frontend/unit/main/main.js')
  </script>

  <script>
    loadScript('/static/frontend/unit/unit.js')
  </script>

  </body>
</html>
