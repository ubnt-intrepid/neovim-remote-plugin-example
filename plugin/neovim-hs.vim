if exists('g:loaded_hello')
  finish
endif
let g:loaded_hello = 1

let s:save_cpo = &cpo
set cpo&vim

let s:script_dir = expand('<sfile>:p:h')
function! s:RequireHello(host) abort
  return jobstart([s:script_dir . '.sh'], {'rpc': v:true})
endfunction

call remote#host#Register('hello', 'x', function('s:RequireHello'))
call remote#host#RegisterPlugin('hello', '0', [
      \ { 'type': 'function', 'name': 'Hello', 'sync': 1, 'opts': {}},
      \ ])

let &cpo = s:save_cpo
unlet s:save_cpo
