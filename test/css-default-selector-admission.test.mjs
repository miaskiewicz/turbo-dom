import test from 'node:test';
import assert from 'node:assert/strict';
import {createEnvironment} from './opt-in-helpers/runtime.mjs';

test('ordinary default CSS ignores invalid rules and non-element terminal pseudo subjects',()=>{
 const {window,document}=createEnvironment('<style>div[data-x=] {color:lime} #target{color:red} ::selection{color:blue} ::-webkit-scrollbar{color:green}</style><div id="target"></div>');
 assert.equal(window.getComputedStyle(document.getElementById('target')).color,'rgb(255, 0, 0)');
});

test('mixed selector validity invalidates a whole rule rather than unrelated styles',()=>{
 const {window,document}=createEnvironment('<style>#target{color:red} #target, span[ {color:blue}</style><div id="target"></div>');
 assert.equal(window.getComputedStyle(document.getElementById('target')).color,'rgb(255, 0, 0)');
});

test('ordinary computed style measures the used width of assigned light content',()=>{
 const {window,document}=createEnvironment('<style>div{width:246px} b{display:block;width:50%;height:20px}</style><div><b>light</b></div>');
 document.querySelector('div').attachShadow({mode:'closed'}).innerHTML='<slot></slot>';
 assert.equal(window.getComputedStyle(document.querySelector('b')).width,'123px');
});

for(const mode of ['open','closed'])test(`external part selectors retain typed refusal for real ${mode} shadow targets`,()=>{
 const {window,document}=createEnvironment('<style>b{color:blue}</style><x-host><b part="label" slot="content">light</b></x-host>');
 const host=document.querySelector('x-host'),root=host.attachShadow({mode});
 root.innerHTML='<slot name="content"></slot><i part="label">shadow</i>';
 assert.equal(window.getComputedStyle(host.firstElementChild).color,'rgb(0, 0, 255)');
 const style=document.createElement('style');
 style.textContent='x-host::part(label){color:red}';
 document.head.appendChild(style);
 // Resolution currently uses a complete scene. A potentially styled shadow
 // row makes both the direct lookup and the broader scene unavailable.
 for(const target of [root.querySelector('i'),host.firstElementChild])
   assert.throws(()=>window.getComputedStyle(target).color,/Route\(UnsupportedComposed\)/);
});
