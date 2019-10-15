# Changelog

### [1.22.1] - 10/14/2019
* [#497](fix): require missing ruby library in rakefile
  downloading files did not work anymore

### [1.22.0] - 10/14/2019
* [](chore): add versioning rake task
  * extract additional rake functionality in it's own file
  * create nicer changelogs
* [#71](chore): workflows with github actions
  rake tasks for linting and creating tagged releases
  fix move of final delivery to folder
  remove unused files when deploying
  fix build for windows
  install missing dependencies for neon build
  make sure correct python version is installed on windows
  vs 2015 install for windows since the default hangs
  rake task to prepare neon builds with cargo config on windows
  remove unused file
  rake task polish
* [](fix): fix rakefile for windows
* [#493](feat) support ticks format
* [](chore): do not execute rake task if an exception happened
* [](chore): refactor rake tasks to build plugins only when necessary"
* [github-xx][fix] correct progress counting (open files)
* [](chore): js library updates
* [](chore) add neon libs to clobber task
  added developer rake task
* [#482](feat) progress reporting in neon lib
  progress for neon task, added js interface
* [github-xx][fix] clean code
* [github-298][feat] version up of toolkit
* [github-xx][fix] fix state controller
* [github-298][feat] update services to fit IPC changes
* [github-298][feat] update toolkit lib
* [github-298][feat] updated standard css
* [github-298][fix] fix standart input
* [github-298][feat] update layout (in scope of sidebar)
* [github-298][refact] added sidebar service
* [github-298][feat] added overwrite method into filters holder
* [github-298][feat] update search view component
* [github-298][feat] update item component (filters tab)
* [github-298][feat] update details component (filters)
* [github-298][feat] controlls panel for filters tab
* [github-298][feat] update filter component
* [github-xx][feat] update recent files dialog
* [github-298][feat] recent fiters dialog
* [github-298][feat] IPC update (render <-> electron)
* [github-xx][fix] reset winhook (for neon) to original
* [github-xx][fix] fix rakefile for windows
* [github-xx][feat] rollback filters service to prev version
* [github-xx][feat] indexer: electron version up to 6
* [github-xx][feat] up to electron 6
* [](chore): add linker function for neon build on linux
* [](chore): add conditional delayed start hook so neon
  so neon works on windows also
  * do not use global neon installation
  * added clean task to npm script
  * added pre-install step for windows to enable delayed electron startup
  * neon integration for linux (fix missing linked symbols
* [](chore): add conditional delayed start hook so neon
  so neon works on windows also
  * do not use global neon installation
  * added clean task to npm script
* [#464](feat): first test for using the newly created neon library for indexing
* [](chore): small updates (neon artifacts, version update of process)
* [](refactor): improve rake task structure
  * added benchmarks
  * better task structure
  * better dependency management for improved performance
  * add neon cli for travis build
* [#464](feat): indexer integration via neon
  * basic event mechanism setup for neon integration
  * add argument to index API for TAG
* [#464](feat) implement progress support and cancel support for indexing
  indexer needs to be able to deliver events in channel
  enable shutdown and progress report
  use log crate for logging
  increase version of indexer cli