Implement the friendly names for the sessions (in the same way as the legacy system does it)

- research lib for the legacy sh friendly name implementation
- write rust tests for the same friendly name implementation in the rust rewrite (para-rs)
- implement the friendly name implementation in the rust rewrite (para-rs)
- run the tests and ensure they pass

Also run they legacy tests (in para-rs/legacytests) and ensure they pass

test_friendly_names.bats
 ✗ FN-1: Create session generates friendly name automatically
   (in test file legacytests/test_friendly_names.bats, line 25)
     `[ -n "$session_dir" ]' failed
   hint: Using 'master' as the name for the initial branch. This default branch name
   hint: is subject to change. To configure the initial branch name to use in all
   hint: of your new repositories, which will suppress this warning, call:
   hint:
   hint:        git config --global init.defaultBranch <name>
   hint:
   hint: Names commonly chosen instead of 'master' are 'main', 'trunk' and
   hint: 'development'. The just-created branch can be renamed via this command:
   hint:
   hint:        git branch -m <name>
   Initialized empty Git repository in /private/var/folders/bl/5pq1htk93_v7n8zv5wsxnzqm0000gn/T/tmp.jwogm9qMFf/.git/
   [master (root-commit) 79892d1] Initial commit
    1 file changed, 1 insertion(+)
    create mode 100644 test-file.py
 ✗ FN-2: List sessions shows friendly names clearly
   (in test file legacytests/test_friendly_names.bats, line 63)
     `[[ "$output" =~ Resume:\ para\ resume\ [a-z]+_[a-z]+_[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]-[0-9][0-9][0-9][0-9][0-9][0-9] ]]' failed
   hint: Using 'master' as the name for the initial branch. This default branch name
   hint: is subject to change. To configure the initial branch name to use in all
   hint: of your new repositories, which will suppress this warning, call:
   hint:
   hint:        git config --global init.defaultBranch <name>
   hint:
   hint: Names commonly chosen instead of 'master' are 'main', 'trunk' and
   hint: 'development'. The just-created branch can be renamed via this command:
   hint:
   hint:        git branch -m <name>
   Initialized empty Git repository in /private/var/folders/bl/5pq1htk93_v7n8zv5wsxnzqm0000gn/T/tmp.BTs3cfGmmM/.git/
   [master (root-commit) 4c9ad2d] Initial commit
    1 file changed, 1 insertion(+)
    create mode 100644 test-file.py
 ✓ FN-3: Resume session by friendly name works
 ✗ FN-4: Auto-detect session with friendly name from worktree
   (in test file legacytests/test_friendly_names.bats, line 112)
     `[ "$status" -eq 0 ]' failed
   hint: Using 'master' as the name for the initial branch. This default branch name
   hint: is subject to change. To configure the initial branch name to use in all
   hint: of your new repositories, which will suppress this warning, call:
   hint:
   hint:        git config --global init.defaultBranch <name>
   hint:
   hint: Names commonly chosen instead of 'master' are 'main', 'trunk' and
   hint: 'development'. The just-created branch can be renamed via this command:
   hint:
   hint:        git branch -m <name>
   Initialized empty Git repository in /private/var/folders/bl/5pq1htk93_v7n8zv5wsxnzqm0000gn/T/tmp.Qn0asQuayb/.git/
   [master (root-commit) 4c9ad2d] Initial commit
    1 file changed, 1 insertion(+)
    create mode 100644 test-file.py
 ✗ FN-5: Cancel session by friendly name works
   (in test file legacytests/test_friendly_names.bats, line 140)
     `[ "$status" -eq 0 ]' failed
   hint: Using 'master' as the name for the initial branch. This default branch name
   hint: is subject to change. To configure the initial branch name to use in all
   hint: of your new repositories, which will suppress this warning, call:
   hint:
   hint:        git config --global init.defaultBranch <name>
   hint:
   hint: Names commonly chosen instead of 'master' are 'main', 'trunk' and
   hint: 'development'. The just-created branch can be renamed via this command:
   hint:
   hint:        git branch -m <name>
   Initialized empty Git repository in /private/var/folders/bl/5pq1htk93_v7n8zv5wsxnzqm0000gn/T/tmp.7eobRM073n/.git/
   [master (root-commit) 4c9ad2d] Initial commit
    1 file changed, 1 insertion(+)
    create mode 100644 test-file.py
 ✗ FN-6: Multiple sessions get different friendly names
   (in test file legacytests/test_friendly_names.bats, line 168)
     `[ "$session_count" -eq 2 ]' failed
   hint: Using 'master' as the name for the initial branch. This default branch name
   hint: is subject to change. To configure the initial branch name to use in all
   hint: of your new repositories, which will suppress this warning, call:
   hint:
   hint:        git config --global init.defaultBranch <name>
   hint:
   hint: Names commonly chosen instead of 'master' are 'main', 'trunk' and
   hint: 'development'. The just-created branch can be renamed via this command:
   hint:
   hint:        git branch -m <name>
   Initialized empty Git repository in /private/var/folders/bl/5pq1htk93_v7n8zv5wsxnzqm0000gn/T/tmp.h0IPEnOLfq/.git/
   [master (root-commit) 4c9ad2d] Initial commit
    1 file changed, 1 insertion(+)
    create mode 100644 test-file.py
 ✗ FN-7: Custom named session still works alongside friendly names
   (in test file legacytests/test_friendly_names.bats, line 200)
     `[ "$status" -eq 0 ]' failed
   hint: Using 'master' as the name for the initial branch. This default branch name
   hint: is subject to change. To configure the initial branch name to use in all
   hint: of your new repositories, which will suppress this warning, call:
   hint:
   hint:        git config --global init.defaultBranch <name>
   hint:
   hint: Names commonly chosen instead of 'master' are 'main', 'trunk' and
   hint: 'development'. The just-created branch can be renamed via this command:
   hint:
   hint:        git branch -m <name>
   Initialized empty Git repository in /private/var/folders/bl/5pq1htk93_v7n8zv5wsxnzqm0000gn/T/tmp.kiMnbg6cdh/.git/
   [master (root-commit) 7694837] Initial commit
    1 file changed, 1 insertion(+)
    create mode 100644 test-file.py
 ✗ FN-8: Backward compatibility with legacy timestamp sessions
   (in test file legacytests/test_friendly_names.bats, line 236)
     `[[ "$output" =~ Session:\ $legacy_session_id\ \(legacy: ]]' failed
   hint: Using 'master' as the name for the initial branch. This default branch name
   hint: is subject to change. To configure the initial branch name to use in all
   hint: of your new repositories, which will suppress this warning, call:
   hint:
   hint:        git config --global init.defaultBranch <name>
   hint:
   hint: Names commonly chosen instead of 'master' are 'main', 'trunk' and
   hint: 'development'. The just-created branch can be renamed via this command:
   hint:
   hint:        git branch -m <name>
   Initialized empty Git repository in /private/var/folders/bl/5pq1htk93_v7n8zv5wsxnzqm0000gn/T/tmp.7kXwu3KTMr/.git/
   [master (root-commit) 7694837] Initial commit
    1 file changed, 1 insertion(+)
    create mode 100644 test-file.py
 ✗ FN-9: Mixed environment - friendly and legacy sessions coexist
   (in test file legacytests/test_friendly_names.bats, line 259)
     `[ "$session_count" -eq 2 ]' failed
   hint: Using 'master' as the name for the initial branch. This default branch name
   hint: is subject to change. To configure the initial branch name to use in all
   hint: of your new repositories, which will suppress this warning, call:
   hint:
   hint:        git config --global init.defaultBranch <name>
   hint:
   hint: Names commonly chosen instead of 'master' are 'main', 'trunk' and
   hint: 'development'. The just-created branch can be renamed via this command:
   hint:
   hint:        git branch -m <name>
   Initialized empty Git repository in /private/var/folders/bl/5pq1htk93_v7n8zv5wsxnzqm0000gn/T/tmp.33aXfb43D7/.git/
   [master (root-commit) 7694837] Initial commit
    1 file changed, 1 insertion(+)
    create mode 100644 test-file.py
 ✗ FN-10: Friendly name generation is consistent across calls
   (in test file legacytests/test_friendly_names.bats, line 293)
     `friendly1=$(generate_friendly_name)' failed with status 127
   hint: Using 'master' as the name for the initial branch. This default branch name
   hint: is subject to change. To configure the initial branch name to use in all
   hint: of your new repositories, which will suppress this warning, call:
   hint:
   hint:        git config --global init.defaultBranch <name>
   hint:
   hint: Names commonly chosen instead of 'master' are 'main', 'trunk' and
   hint: 'development'. The just-created branch can be renamed via this command:
   hint:
   hint:        git branch -m <name>
   Initialized empty Git repository in /private/var/folders/bl/5pq1htk93_v7n8zv5wsxnzqm0000gn/T/tmp.JulTeoCYg2/.git/
   [master (root-commit) 7694837] Initial commit
    1 file changed, 1 insertion(+)
    create mode 100644 test-file.py
   /Users/marius.wichtner/Documents/git/para/para-rs/legacytests/test_friendly_names.bats: line 293: generate_friendly_name: command not found