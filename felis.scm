(require (prefix-in helix. "helix/commands.scm"))

(require "helix/editor.scm")

(provide felis-open
         felis-file-browser
         felis-file-browser-cwd)

;; Utils

(define (current-doc-path)
  (let* ([focus (editor-focus)]
         [focus-doc-id (editor->doc-id focus)])
    (editor-document->path focus-doc-id)))

;; Commands

(define (felis-open)
  (let ((path ( ~> (open-input-file "/tmp/felis-open.txt") (read-port-to-string))))
    (helix.open path)))

(define (felis-file-browser felis-bin browser-bin)
  (helix.run-shell-command felis-bin "open-browser" "-l" "--steel" browser-bin))

(define (felis-file-browser-cwd felis-bin browser-bin)
  (let ((current-file (current-doc-path)))
    (helix.run-shell-command felis-bin "open-browser" "-l" "--steel" browser-bin (trim-end-matches current-file (file-name current-file)))))


