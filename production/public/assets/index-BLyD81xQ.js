(function(){const e=document.createElement("link").relList;if(e&&e.supports&&e.supports("modulepreload"))return;for(const l of document.querySelectorAll('link[rel="modulepreload"]'))s(l);new MutationObserver(l=>{for(const c of l)if(c.type==="childList")for(const d of c.addedNodes)d.tagName==="LINK"&&d.rel==="modulepreload"&&s(d)}).observe(document,{childList:!0,subtree:!0});function i(l){const c={};return l.integrity&&(c.integrity=l.integrity),l.referrerPolicy&&(c.referrerPolicy=l.referrerPolicy),l.crossOrigin==="use-credentials"?c.credentials="include":l.crossOrigin==="anonymous"?c.credentials="omit":c.credentials="same-origin",c}function s(l){if(l.ep)return;l.ep=!0;const c=i(l);fetch(l.href,c)}})();function K_(o){return o&&o.__esModule&&Object.prototype.hasOwnProperty.call(o,"default")?o.default:o}var od={exports:{}},Io={};/**
 * @license React
 * react-jsx-runtime.production.js
 *
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */var I0;function Hy(){if(I0)return Io;I0=1;var o=Symbol.for("react.transitional.element"),e=Symbol.for("react.fragment");function i(s,l,c){var d=null;if(c!==void 0&&(d=""+c),l.key!==void 0&&(d=""+l.key),"key"in l){c={};for(var p in l)p!=="key"&&(c[p]=l[p])}else c=l;return l=c.ref,{$$typeof:o,type:s,key:d,ref:l!==void 0?l:null,props:c}}return Io.Fragment=e,Io.jsx=i,Io.jsxs=i,Io}var F0;function Gy(){return F0||(F0=1,od.exports=Hy()),od.exports}var O=Gy(),ld={exports:{}},rt={};/**
 * @license React
 * react.production.js
 *
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */var z0;function Vy(){if(z0)return rt;z0=1;var o=Symbol.for("react.transitional.element"),e=Symbol.for("react.portal"),i=Symbol.for("react.fragment"),s=Symbol.for("react.strict_mode"),l=Symbol.for("react.profiler"),c=Symbol.for("react.consumer"),d=Symbol.for("react.context"),p=Symbol.for("react.forward_ref"),m=Symbol.for("react.suspense"),h=Symbol.for("react.memo"),v=Symbol.for("react.lazy"),y=Symbol.for("react.activity"),g=Symbol.iterator;function x(I){return I===null||typeof I!="object"?null:(I=g&&I[g]||I["@@iterator"],typeof I=="function"?I:null)}var E={isMounted:function(){return!1},enqueueForceUpdate:function(){},enqueueReplaceState:function(){},enqueueSetState:function(){}},w=Object.assign,b={};function S(I,Y,ve){this.props=I,this.context=Y,this.refs=b,this.updater=ve||E}S.prototype.isReactComponent={},S.prototype.setState=function(I,Y){if(typeof I!="object"&&typeof I!="function"&&I!=null)throw Error("takes an object of state variables to update or a function which returns an object of state variables.");this.updater.enqueueSetState(this,I,Y,"setState")},S.prototype.forceUpdate=function(I){this.updater.enqueueForceUpdate(this,I,"forceUpdate")};function C(){}C.prototype=S.prototype;function U(I,Y,ve){this.props=I,this.context=Y,this.refs=b,this.updater=ve||E}var N=U.prototype=new C;N.constructor=U,w(N,S.prototype),N.isPureReactComponent=!0;var V=Array.isArray;function H(){}var F={H:null,A:null,T:null,S:null},T=Object.prototype.hasOwnProperty;function D(I,Y,ve){var Re=ve.ref;return{$$typeof:o,type:I,key:Y,ref:Re!==void 0?Re:null,props:ve}}function le(I,Y){return D(I.type,Y,I.props)}function G(I){return typeof I=="object"&&I!==null&&I.$$typeof===o}function te(I){var Y={"=":"=0",":":"=2"};return"$"+I.replace(/[=:]/g,function(ve){return Y[ve]})}var se=/\/+/g;function ue(I,Y){return typeof I=="object"&&I!==null&&I.key!=null?te(""+I.key):Y.toString(36)}function ee(I){switch(I.status){case"fulfilled":return I.value;case"rejected":throw I.reason;default:switch(typeof I.status=="string"?I.then(H,H):(I.status="pending",I.then(function(Y){I.status==="pending"&&(I.status="fulfilled",I.value=Y)},function(Y){I.status==="pending"&&(I.status="rejected",I.reason=Y)})),I.status){case"fulfilled":return I.value;case"rejected":throw I.reason}}throw I}function P(I,Y,ve,Re,Fe){var ie=typeof I;(ie==="undefined"||ie==="boolean")&&(I=null);var xe=!1;if(I===null)xe=!0;else switch(ie){case"bigint":case"string":case"number":xe=!0;break;case"object":switch(I.$$typeof){case o:case e:xe=!0;break;case v:return xe=I._init,P(xe(I._payload),Y,ve,Re,Fe)}}if(xe)return Fe=Fe(I),xe=Re===""?"."+ue(I,0):Re,V(Fe)?(ve="",xe!=null&&(ve=xe.replace(se,"$&/")+"/"),P(Fe,Y,ve,"",function(Ke){return Ke})):Fe!=null&&(G(Fe)&&(Fe=le(Fe,ve+(Fe.key==null||I&&I.key===Fe.key?"":(""+Fe.key).replace(se,"$&/")+"/")+xe)),Y.push(Fe)),1;xe=0;var Te=Re===""?".":Re+":";if(V(I))for(var ke=0;ke<I.length;ke++)Re=I[ke],ie=Te+ue(Re,ke),xe+=P(Re,Y,ve,ie,Fe);else if(ke=x(I),typeof ke=="function")for(I=ke.call(I),ke=0;!(Re=I.next()).done;)Re=Re.value,ie=Te+ue(Re,ke++),xe+=P(Re,Y,ve,ie,Fe);else if(ie==="object"){if(typeof I.then=="function")return P(ee(I),Y,ve,Re,Fe);throw Y=String(I),Error("Objects are not valid as a React child (found: "+(Y==="[object Object]"?"object with keys {"+Object.keys(I).join(", ")+"}":Y)+"). If you meant to render a collection of children, use an array instead.")}return xe}function z(I,Y,ve){if(I==null)return I;var Re=[],Fe=0;return P(I,Re,"","",function(ie){return Y.call(ve,ie,Fe++)}),Re}function ce(I){if(I._status===-1){var Y=I._result;Y=Y(),Y.then(function(ve){(I._status===0||I._status===-1)&&(I._status=1,I._result=ve)},function(ve){(I._status===0||I._status===-1)&&(I._status=2,I._result=ve)}),I._status===-1&&(I._status=0,I._result=Y)}if(I._status===1)return I._result.default;throw I._result}var pe=typeof reportError=="function"?reportError:function(I){if(typeof window=="object"&&typeof window.ErrorEvent=="function"){var Y=new window.ErrorEvent("error",{bubbles:!0,cancelable:!0,message:typeof I=="object"&&I!==null&&typeof I.message=="string"?String(I.message):String(I),error:I});if(!window.dispatchEvent(Y))return}else if(typeof process=="object"&&typeof process.emit=="function"){process.emit("uncaughtException",I);return}console.error(I)},Ee={map:z,forEach:function(I,Y,ve){z(I,function(){Y.apply(this,arguments)},ve)},count:function(I){var Y=0;return z(I,function(){Y++}),Y},toArray:function(I){return z(I,function(Y){return Y})||[]},only:function(I){if(!G(I))throw Error("React.Children.only expected to receive a single React element child.");return I}};return rt.Activity=y,rt.Children=Ee,rt.Component=S,rt.Fragment=i,rt.Profiler=l,rt.PureComponent=U,rt.StrictMode=s,rt.Suspense=m,rt.__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE=F,rt.__COMPILER_RUNTIME={__proto__:null,c:function(I){return F.H.useMemoCache(I)}},rt.cache=function(I){return function(){return I.apply(null,arguments)}},rt.cacheSignal=function(){return null},rt.cloneElement=function(I,Y,ve){if(I==null)throw Error("The argument must be a React element, but you passed "+I+".");var Re=w({},I.props),Fe=I.key;if(Y!=null)for(ie in Y.key!==void 0&&(Fe=""+Y.key),Y)!T.call(Y,ie)||ie==="key"||ie==="__self"||ie==="__source"||ie==="ref"&&Y.ref===void 0||(Re[ie]=Y[ie]);var ie=arguments.length-2;if(ie===1)Re.children=ve;else if(1<ie){for(var xe=Array(ie),Te=0;Te<ie;Te++)xe[Te]=arguments[Te+2];Re.children=xe}return D(I.type,Fe,Re)},rt.createContext=function(I){return I={$$typeof:d,_currentValue:I,_currentValue2:I,_threadCount:0,Provider:null,Consumer:null},I.Provider=I,I.Consumer={$$typeof:c,_context:I},I},rt.createElement=function(I,Y,ve){var Re,Fe={},ie=null;if(Y!=null)for(Re in Y.key!==void 0&&(ie=""+Y.key),Y)T.call(Y,Re)&&Re!=="key"&&Re!=="__self"&&Re!=="__source"&&(Fe[Re]=Y[Re]);var xe=arguments.length-2;if(xe===1)Fe.children=ve;else if(1<xe){for(var Te=Array(xe),ke=0;ke<xe;ke++)Te[ke]=arguments[ke+2];Fe.children=Te}if(I&&I.defaultProps)for(Re in xe=I.defaultProps,xe)Fe[Re]===void 0&&(Fe[Re]=xe[Re]);return D(I,ie,Fe)},rt.createRef=function(){return{current:null}},rt.forwardRef=function(I){return{$$typeof:p,render:I}},rt.isValidElement=G,rt.lazy=function(I){return{$$typeof:v,_payload:{_status:-1,_result:I},_init:ce}},rt.memo=function(I,Y){return{$$typeof:h,type:I,compare:Y===void 0?null:Y}},rt.startTransition=function(I){var Y=F.T,ve={};F.T=ve;try{var Re=I(),Fe=F.S;Fe!==null&&Fe(ve,Re),typeof Re=="object"&&Re!==null&&typeof Re.then=="function"&&Re.then(H,pe)}catch(ie){pe(ie)}finally{Y!==null&&ve.types!==null&&(Y.types=ve.types),F.T=Y}},rt.unstable_useCacheRefresh=function(){return F.H.useCacheRefresh()},rt.use=function(I){return F.H.use(I)},rt.useActionState=function(I,Y,ve){return F.H.useActionState(I,Y,ve)},rt.useCallback=function(I,Y){return F.H.useCallback(I,Y)},rt.useContext=function(I){return F.H.useContext(I)},rt.useDebugValue=function(){},rt.useDeferredValue=function(I,Y){return F.H.useDeferredValue(I,Y)},rt.useEffect=function(I,Y){return F.H.useEffect(I,Y)},rt.useEffectEvent=function(I){return F.H.useEffectEvent(I)},rt.useId=function(){return F.H.useId()},rt.useImperativeHandle=function(I,Y,ve){return F.H.useImperativeHandle(I,Y,ve)},rt.useInsertionEffect=function(I,Y){return F.H.useInsertionEffect(I,Y)},rt.useLayoutEffect=function(I,Y){return F.H.useLayoutEffect(I,Y)},rt.useMemo=function(I,Y){return F.H.useMemo(I,Y)},rt.useOptimistic=function(I,Y){return F.H.useOptimistic(I,Y)},rt.useReducer=function(I,Y,ve){return F.H.useReducer(I,Y,ve)},rt.useRef=function(I){return F.H.useRef(I)},rt.useState=function(I){return F.H.useState(I)},rt.useSyncExternalStore=function(I,Y,ve){return F.H.useSyncExternalStore(I,Y,ve)},rt.useTransition=function(){return F.H.useTransition()},rt.version="19.2.4",rt}var B0;function Vh(){return B0||(B0=1,ld.exports=Vy()),ld.exports}var Wt=Vh();const ky=K_(Wt);var cd={exports:{}},Fo={},ud={exports:{}},fd={};/**
 * @license React
 * scheduler.production.js
 *
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */var H0;function Xy(){return H0||(H0=1,(function(o){function e(P,z){var ce=P.length;P.push(z);e:for(;0<ce;){var pe=ce-1>>>1,Ee=P[pe];if(0<l(Ee,z))P[pe]=z,P[ce]=Ee,ce=pe;else break e}}function i(P){return P.length===0?null:P[0]}function s(P){if(P.length===0)return null;var z=P[0],ce=P.pop();if(ce!==z){P[0]=ce;e:for(var pe=0,Ee=P.length,I=Ee>>>1;pe<I;){var Y=2*(pe+1)-1,ve=P[Y],Re=Y+1,Fe=P[Re];if(0>l(ve,ce))Re<Ee&&0>l(Fe,ve)?(P[pe]=Fe,P[Re]=ce,pe=Re):(P[pe]=ve,P[Y]=ce,pe=Y);else if(Re<Ee&&0>l(Fe,ce))P[pe]=Fe,P[Re]=ce,pe=Re;else break e}}return z}function l(P,z){var ce=P.sortIndex-z.sortIndex;return ce!==0?ce:P.id-z.id}if(o.unstable_now=void 0,typeof performance=="object"&&typeof performance.now=="function"){var c=performance;o.unstable_now=function(){return c.now()}}else{var d=Date,p=d.now();o.unstable_now=function(){return d.now()-p}}var m=[],h=[],v=1,y=null,g=3,x=!1,E=!1,w=!1,b=!1,S=typeof setTimeout=="function"?setTimeout:null,C=typeof clearTimeout=="function"?clearTimeout:null,U=typeof setImmediate<"u"?setImmediate:null;function N(P){for(var z=i(h);z!==null;){if(z.callback===null)s(h);else if(z.startTime<=P)s(h),z.sortIndex=z.expirationTime,e(m,z);else break;z=i(h)}}function V(P){if(w=!1,N(P),!E)if(i(m)!==null)E=!0,H||(H=!0,te());else{var z=i(h);z!==null&&ee(V,z.startTime-P)}}var H=!1,F=-1,T=5,D=-1;function le(){return b?!0:!(o.unstable_now()-D<T)}function G(){if(b=!1,H){var P=o.unstable_now();D=P;var z=!0;try{e:{E=!1,w&&(w=!1,C(F),F=-1),x=!0;var ce=g;try{t:{for(N(P),y=i(m);y!==null&&!(y.expirationTime>P&&le());){var pe=y.callback;if(typeof pe=="function"){y.callback=null,g=y.priorityLevel;var Ee=pe(y.expirationTime<=P);if(P=o.unstable_now(),typeof Ee=="function"){y.callback=Ee,N(P),z=!0;break t}y===i(m)&&s(m),N(P)}else s(m);y=i(m)}if(y!==null)z=!0;else{var I=i(h);I!==null&&ee(V,I.startTime-P),z=!1}}break e}finally{y=null,g=ce,x=!1}z=void 0}}finally{z?te():H=!1}}}var te;if(typeof U=="function")te=function(){U(G)};else if(typeof MessageChannel<"u"){var se=new MessageChannel,ue=se.port2;se.port1.onmessage=G,te=function(){ue.postMessage(null)}}else te=function(){S(G,0)};function ee(P,z){F=S(function(){P(o.unstable_now())},z)}o.unstable_IdlePriority=5,o.unstable_ImmediatePriority=1,o.unstable_LowPriority=4,o.unstable_NormalPriority=3,o.unstable_Profiling=null,o.unstable_UserBlockingPriority=2,o.unstable_cancelCallback=function(P){P.callback=null},o.unstable_forceFrameRate=function(P){0>P||125<P?console.error("forceFrameRate takes a positive int between 0 and 125, forcing frame rates higher than 125 fps is not supported"):T=0<P?Math.floor(1e3/P):5},o.unstable_getCurrentPriorityLevel=function(){return g},o.unstable_next=function(P){switch(g){case 1:case 2:case 3:var z=3;break;default:z=g}var ce=g;g=z;try{return P()}finally{g=ce}},o.unstable_requestPaint=function(){b=!0},o.unstable_runWithPriority=function(P,z){switch(P){case 1:case 2:case 3:case 4:case 5:break;default:P=3}var ce=g;g=P;try{return z()}finally{g=ce}},o.unstable_scheduleCallback=function(P,z,ce){var pe=o.unstable_now();switch(typeof ce=="object"&&ce!==null?(ce=ce.delay,ce=typeof ce=="number"&&0<ce?pe+ce:pe):ce=pe,P){case 1:var Ee=-1;break;case 2:Ee=250;break;case 5:Ee=1073741823;break;case 4:Ee=1e4;break;default:Ee=5e3}return Ee=ce+Ee,P={id:v++,callback:z,priorityLevel:P,startTime:ce,expirationTime:Ee,sortIndex:-1},ce>pe?(P.sortIndex=ce,e(h,P),i(m)===null&&P===i(h)&&(w?(C(F),F=-1):w=!0,ee(V,ce-pe))):(P.sortIndex=Ee,e(m,P),E||x||(E=!0,H||(H=!0,te()))),P},o.unstable_shouldYield=le,o.unstable_wrapCallback=function(P){var z=g;return function(){var ce=g;g=z;try{return P.apply(this,arguments)}finally{g=ce}}}})(fd)),fd}var G0;function jy(){return G0||(G0=1,ud.exports=Xy()),ud.exports}var dd={exports:{}},Rn={};/**
 * @license React
 * react-dom.production.js
 *
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */var V0;function Wy(){if(V0)return Rn;V0=1;var o=Vh();function e(m){var h="https://react.dev/errors/"+m;if(1<arguments.length){h+="?args[]="+encodeURIComponent(arguments[1]);for(var v=2;v<arguments.length;v++)h+="&args[]="+encodeURIComponent(arguments[v])}return"Minified React error #"+m+"; visit "+h+" for the full message or use the non-minified dev environment for full errors and additional helpful warnings."}function i(){}var s={d:{f:i,r:function(){throw Error(e(522))},D:i,C:i,L:i,m:i,X:i,S:i,M:i},p:0,findDOMNode:null},l=Symbol.for("react.portal");function c(m,h,v){var y=3<arguments.length&&arguments[3]!==void 0?arguments[3]:null;return{$$typeof:l,key:y==null?null:""+y,children:m,containerInfo:h,implementation:v}}var d=o.__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE;function p(m,h){if(m==="font")return"";if(typeof h=="string")return h==="use-credentials"?h:""}return Rn.__DOM_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE=s,Rn.createPortal=function(m,h){var v=2<arguments.length&&arguments[2]!==void 0?arguments[2]:null;if(!h||h.nodeType!==1&&h.nodeType!==9&&h.nodeType!==11)throw Error(e(299));return c(m,h,null,v)},Rn.flushSync=function(m){var h=d.T,v=s.p;try{if(d.T=null,s.p=2,m)return m()}finally{d.T=h,s.p=v,s.d.f()}},Rn.preconnect=function(m,h){typeof m=="string"&&(h?(h=h.crossOrigin,h=typeof h=="string"?h==="use-credentials"?h:"":void 0):h=null,s.d.C(m,h))},Rn.prefetchDNS=function(m){typeof m=="string"&&s.d.D(m)},Rn.preinit=function(m,h){if(typeof m=="string"&&h&&typeof h.as=="string"){var v=h.as,y=p(v,h.crossOrigin),g=typeof h.integrity=="string"?h.integrity:void 0,x=typeof h.fetchPriority=="string"?h.fetchPriority:void 0;v==="style"?s.d.S(m,typeof h.precedence=="string"?h.precedence:void 0,{crossOrigin:y,integrity:g,fetchPriority:x}):v==="script"&&s.d.X(m,{crossOrigin:y,integrity:g,fetchPriority:x,nonce:typeof h.nonce=="string"?h.nonce:void 0})}},Rn.preinitModule=function(m,h){if(typeof m=="string")if(typeof h=="object"&&h!==null){if(h.as==null||h.as==="script"){var v=p(h.as,h.crossOrigin);s.d.M(m,{crossOrigin:v,integrity:typeof h.integrity=="string"?h.integrity:void 0,nonce:typeof h.nonce=="string"?h.nonce:void 0})}}else h==null&&s.d.M(m)},Rn.preload=function(m,h){if(typeof m=="string"&&typeof h=="object"&&h!==null&&typeof h.as=="string"){var v=h.as,y=p(v,h.crossOrigin);s.d.L(m,v,{crossOrigin:y,integrity:typeof h.integrity=="string"?h.integrity:void 0,nonce:typeof h.nonce=="string"?h.nonce:void 0,type:typeof h.type=="string"?h.type:void 0,fetchPriority:typeof h.fetchPriority=="string"?h.fetchPriority:void 0,referrerPolicy:typeof h.referrerPolicy=="string"?h.referrerPolicy:void 0,imageSrcSet:typeof h.imageSrcSet=="string"?h.imageSrcSet:void 0,imageSizes:typeof h.imageSizes=="string"?h.imageSizes:void 0,media:typeof h.media=="string"?h.media:void 0})}},Rn.preloadModule=function(m,h){if(typeof m=="string")if(h){var v=p(h.as,h.crossOrigin);s.d.m(m,{as:typeof h.as=="string"&&h.as!=="script"?h.as:void 0,crossOrigin:v,integrity:typeof h.integrity=="string"?h.integrity:void 0})}else s.d.m(m)},Rn.requestFormReset=function(m){s.d.r(m)},Rn.unstable_batchedUpdates=function(m,h){return m(h)},Rn.useFormState=function(m,h,v){return d.H.useFormState(m,h,v)},Rn.useFormStatus=function(){return d.H.useHostTransitionStatus()},Rn.version="19.2.4",Rn}var k0;function qy(){if(k0)return dd.exports;k0=1;function o(){if(!(typeof __REACT_DEVTOOLS_GLOBAL_HOOK__>"u"||typeof __REACT_DEVTOOLS_GLOBAL_HOOK__.checkDCE!="function"))try{__REACT_DEVTOOLS_GLOBAL_HOOK__.checkDCE(o)}catch(e){console.error(e)}}return o(),dd.exports=Wy(),dd.exports}/**
 * @license React
 * react-dom-client.production.js
 *
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */var X0;function Yy(){if(X0)return Fo;X0=1;var o=jy(),e=Vh(),i=qy();function s(t){var n="https://react.dev/errors/"+t;if(1<arguments.length){n+="?args[]="+encodeURIComponent(arguments[1]);for(var a=2;a<arguments.length;a++)n+="&args[]="+encodeURIComponent(arguments[a])}return"Minified React error #"+t+"; visit "+n+" for the full message or use the non-minified dev environment for full errors and additional helpful warnings."}function l(t){return!(!t||t.nodeType!==1&&t.nodeType!==9&&t.nodeType!==11)}function c(t){var n=t,a=t;if(t.alternate)for(;n.return;)n=n.return;else{t=n;do n=t,(n.flags&4098)!==0&&(a=n.return),t=n.return;while(t)}return n.tag===3?a:null}function d(t){if(t.tag===13){var n=t.memoizedState;if(n===null&&(t=t.alternate,t!==null&&(n=t.memoizedState)),n!==null)return n.dehydrated}return null}function p(t){if(t.tag===31){var n=t.memoizedState;if(n===null&&(t=t.alternate,t!==null&&(n=t.memoizedState)),n!==null)return n.dehydrated}return null}function m(t){if(c(t)!==t)throw Error(s(188))}function h(t){var n=t.alternate;if(!n){if(n=c(t),n===null)throw Error(s(188));return n!==t?null:t}for(var a=t,r=n;;){var u=a.return;if(u===null)break;var f=u.alternate;if(f===null){if(r=u.return,r!==null){a=r;continue}break}if(u.child===f.child){for(f=u.child;f;){if(f===a)return m(u),t;if(f===r)return m(u),n;f=f.sibling}throw Error(s(188))}if(a.return!==r.return)a=u,r=f;else{for(var _=!1,A=u.child;A;){if(A===a){_=!0,a=u,r=f;break}if(A===r){_=!0,r=u,a=f;break}A=A.sibling}if(!_){for(A=f.child;A;){if(A===a){_=!0,a=f,r=u;break}if(A===r){_=!0,r=f,a=u;break}A=A.sibling}if(!_)throw Error(s(189))}}if(a.alternate!==r)throw Error(s(190))}if(a.tag!==3)throw Error(s(188));return a.stateNode.current===a?t:n}function v(t){var n=t.tag;if(n===5||n===26||n===27||n===6)return t;for(t=t.child;t!==null;){if(n=v(t),n!==null)return n;t=t.sibling}return null}var y=Object.assign,g=Symbol.for("react.element"),x=Symbol.for("react.transitional.element"),E=Symbol.for("react.portal"),w=Symbol.for("react.fragment"),b=Symbol.for("react.strict_mode"),S=Symbol.for("react.profiler"),C=Symbol.for("react.consumer"),U=Symbol.for("react.context"),N=Symbol.for("react.forward_ref"),V=Symbol.for("react.suspense"),H=Symbol.for("react.suspense_list"),F=Symbol.for("react.memo"),T=Symbol.for("react.lazy"),D=Symbol.for("react.activity"),le=Symbol.for("react.memo_cache_sentinel"),G=Symbol.iterator;function te(t){return t===null||typeof t!="object"?null:(t=G&&t[G]||t["@@iterator"],typeof t=="function"?t:null)}var se=Symbol.for("react.client.reference");function ue(t){if(t==null)return null;if(typeof t=="function")return t.$$typeof===se?null:t.displayName||t.name||null;if(typeof t=="string")return t;switch(t){case w:return"Fragment";case S:return"Profiler";case b:return"StrictMode";case V:return"Suspense";case H:return"SuspenseList";case D:return"Activity"}if(typeof t=="object")switch(t.$$typeof){case E:return"Portal";case U:return t.displayName||"Context";case C:return(t._context.displayName||"Context")+".Consumer";case N:var n=t.render;return t=t.displayName,t||(t=n.displayName||n.name||"",t=t!==""?"ForwardRef("+t+")":"ForwardRef"),t;case F:return n=t.displayName||null,n!==null?n:ue(t.type)||"Memo";case T:n=t._payload,t=t._init;try{return ue(t(n))}catch{}}return null}var ee=Array.isArray,P=e.__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE,z=i.__DOM_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE,ce={pending:!1,data:null,method:null,action:null},pe=[],Ee=-1;function I(t){return{current:t}}function Y(t){0>Ee||(t.current=pe[Ee],pe[Ee]=null,Ee--)}function ve(t,n){Ee++,pe[Ee]=t.current,t.current=n}var Re=I(null),Fe=I(null),ie=I(null),xe=I(null);function Te(t,n){switch(ve(ie,n),ve(Fe,t),ve(Re,null),n.nodeType){case 9:case 11:t=(t=n.documentElement)&&(t=t.namespaceURI)?s0(t):0;break;default:if(t=n.tagName,n=n.namespaceURI)n=s0(n),t=r0(n,t);else switch(t){case"svg":t=1;break;case"math":t=2;break;default:t=0}}Y(Re),ve(Re,t)}function ke(){Y(Re),Y(Fe),Y(ie)}function Ke(t){t.memoizedState!==null&&ve(xe,t);var n=Re.current,a=r0(n,t.type);n!==a&&(ve(Fe,t),ve(Re,a))}function $e(t){Fe.current===t&&(Y(Re),Y(Fe)),xe.current===t&&(Y(xe),Uo._currentValue=ce)}var $t,xt;function mt(t){if($t===void 0)try{throw Error()}catch(a){var n=a.stack.trim().match(/\n( *(at )?)/);$t=n&&n[1]||"",xt=-1<a.stack.indexOf(`
    at`)?" (<anonymous>)":-1<a.stack.indexOf("@")?"@unknown:0:0":""}return`
`+$t+t+xt}var Nt=!1;function ot(t,n){if(!t||Nt)return"";Nt=!0;var a=Error.prepareStackTrace;Error.prepareStackTrace=void 0;try{var r={DetermineComponentFrameRoot:function(){try{if(n){var _e=function(){throw Error()};if(Object.defineProperty(_e.prototype,"props",{set:function(){throw Error()}}),typeof Reflect=="object"&&Reflect.construct){try{Reflect.construct(_e,[])}catch(oe){var ae=oe}Reflect.construct(t,[],_e)}else{try{_e.call()}catch(oe){ae=oe}t.call(_e.prototype)}}else{try{throw Error()}catch(oe){ae=oe}(_e=t())&&typeof _e.catch=="function"&&_e.catch(function(){})}}catch(oe){if(oe&&ae&&typeof oe.stack=="string")return[oe.stack,ae.stack]}return[null,null]}};r.DetermineComponentFrameRoot.displayName="DetermineComponentFrameRoot";var u=Object.getOwnPropertyDescriptor(r.DetermineComponentFrameRoot,"name");u&&u.configurable&&Object.defineProperty(r.DetermineComponentFrameRoot,"name",{value:"DetermineComponentFrameRoot"});var f=r.DetermineComponentFrameRoot(),_=f[0],A=f[1];if(_&&A){var B=_.split(`
`),$=A.split(`
`);for(u=r=0;r<B.length&&!B[r].includes("DetermineComponentFrameRoot");)r++;for(;u<$.length&&!$[u].includes("DetermineComponentFrameRoot");)u++;if(r===B.length||u===$.length)for(r=B.length-1,u=$.length-1;1<=r&&0<=u&&B[r]!==$[u];)u--;for(;1<=r&&0<=u;r--,u--)if(B[r]!==$[u]){if(r!==1||u!==1)do if(r--,u--,0>u||B[r]!==$[u]){var he=`
`+B[r].replace(" at new "," at ");return t.displayName&&he.includes("<anonymous>")&&(he=he.replace("<anonymous>",t.displayName)),he}while(1<=r&&0<=u);break}}}finally{Nt=!1,Error.prepareStackTrace=a}return(a=t?t.displayName||t.name:"")?mt(a):""}function Qt(t,n){switch(t.tag){case 26:case 27:case 5:return mt(t.type);case 16:return mt("Lazy");case 13:return t.child!==n&&n!==null?mt("Suspense Fallback"):mt("Suspense");case 19:return mt("SuspenseList");case 0:case 15:return ot(t.type,!1);case 11:return ot(t.type.render,!1);case 1:return ot(t.type,!0);case 31:return mt("Activity");default:return""}}function k(t){try{var n="",a=null;do n+=Qt(t,a),a=t,t=t.return;while(t);return n}catch(r){return`
Error generating stack: `+r.message+`
`+r.stack}}var qt=Object.prototype.hasOwnProperty,Mt=o.unstable_scheduleCallback,Lt=o.unstable_cancelCallback,We=o.unstable_shouldYield,L=o.unstable_requestPaint,M=o.unstable_now,q=o.unstable_getCurrentPriorityLevel,me=o.unstable_ImmediatePriority,ye=o.unstable_UserBlockingPriority,de=o.unstable_NormalPriority,Xe=o.unstable_LowPriority,Ce=o.unstable_IdlePriority,Ze=o.log,et=o.unstable_setDisableYieldValue,Me=null,Se=null;function Oe(t){if(typeof Ze=="function"&&et(t),Se&&typeof Se.setStrictMode=="function")try{Se.setStrictMode(Me,t)}catch{}}var Le=Math.clz32?Math.clz32:W,Pe=Math.log,ut=Math.LN2;function W(t){return t>>>=0,t===0?32:31-(Pe(t)/ut|0)|0}var we=256,Ae=262144,Ie=4194304;function be(t){var n=t&42;if(n!==0)return n;switch(t&-t){case 1:return 1;case 2:return 2;case 4:return 4;case 8:return 8;case 16:return 16;case 32:return 32;case 64:return 64;case 128:return 128;case 256:case 512:case 1024:case 2048:case 4096:case 8192:case 16384:case 32768:case 65536:case 131072:return t&261888;case 262144:case 524288:case 1048576:case 2097152:return t&3932160;case 4194304:case 8388608:case 16777216:case 33554432:return t&62914560;case 67108864:return 67108864;case 134217728:return 134217728;case 268435456:return 268435456;case 536870912:return 536870912;case 1073741824:return 0;default:return t}}function fe(t,n,a){var r=t.pendingLanes;if(r===0)return 0;var u=0,f=t.suspendedLanes,_=t.pingedLanes;t=t.warmLanes;var A=r&134217727;return A!==0?(r=A&~f,r!==0?u=be(r):(_&=A,_!==0?u=be(_):a||(a=A&~t,a!==0&&(u=be(a))))):(A=r&~f,A!==0?u=be(A):_!==0?u=be(_):a||(a=r&~t,a!==0&&(u=be(a)))),u===0?0:n!==0&&n!==u&&(n&f)===0&&(f=u&-u,a=n&-n,f>=a||f===32&&(a&4194048)!==0)?n:u}function Be(t,n){return(t.pendingLanes&~(t.suspendedLanes&~t.pingedLanes)&n)===0}function nt(t,n){switch(t){case 1:case 2:case 4:case 8:case 64:return n+250;case 16:case 32:case 128:case 256:case 512:case 1024:case 2048:case 4096:case 8192:case 16384:case 32768:case 65536:case 131072:case 262144:case 524288:case 1048576:case 2097152:return n+5e3;case 4194304:case 8388608:case 16777216:case 33554432:return-1;case 67108864:case 134217728:case 268435456:case 536870912:case 1073741824:return-1;default:return-1}}function Pt(){var t=Ie;return Ie<<=1,(Ie&62914560)===0&&(Ie=4194304),t}function Et(t){for(var n=[],a=0;31>a;a++)n.push(t);return n}function Nn(t,n){t.pendingLanes|=n,n!==268435456&&(t.suspendedLanes=0,t.pingedLanes=0,t.warmLanes=0)}function xi(t,n,a,r,u,f){var _=t.pendingLanes;t.pendingLanes=a,t.suspendedLanes=0,t.pingedLanes=0,t.warmLanes=0,t.expiredLanes&=a,t.entangledLanes&=a,t.errorRecoveryDisabledLanes&=a,t.shellSuspendCounter=0;var A=t.entanglements,B=t.expirationTimes,$=t.hiddenUpdates;for(a=_&~a;0<a;){var he=31-Le(a),_e=1<<he;A[he]=0,B[he]=-1;var ae=$[he];if(ae!==null)for($[he]=null,he=0;he<ae.length;he++){var oe=ae[he];oe!==null&&(oe.lane&=-536870913)}a&=~_e}r!==0&&Wr(t,r,0),f!==0&&u===0&&t.tag!==0&&(t.suspendedLanes|=f&~(_&~n))}function Wr(t,n,a){t.pendingLanes|=n,t.suspendedLanes&=~n;var r=31-Le(n);t.entangledLanes|=n,t.entanglements[r]=t.entanglements[r]|1073741824|a&261930}function zs(t,n){var a=t.entangledLanes|=n;for(t=t.entanglements;a;){var r=31-Le(a),u=1<<r;u&n|t[r]&n&&(t[r]|=n),a&=~u}}function el(t,n){var a=n&-n;return a=(a&42)!==0?1:Bs(a),(a&(t.suspendedLanes|n))!==0?0:a}function Bs(t){switch(t){case 2:t=1;break;case 8:t=4;break;case 32:t=16;break;case 256:case 512:case 1024:case 2048:case 4096:case 8192:case 16384:case 32768:case 65536:case 131072:case 262144:case 524288:case 1048576:case 2097152:case 4194304:case 8388608:case 16777216:case 33554432:t=128;break;case 268435456:t=134217728;break;default:t=0}return t}function Hs(t){return t&=-t,2<t?8<t?(t&134217727)!==0?32:268435456:8:2}function Ui(){var t=z.p;return t!==0?t:(t=window.event,t===void 0?32:C0(t.type))}function Gs(t,n){var a=z.p;try{return z.p=t,n()}finally{z.p=a}}var yi=Math.random().toString(36).slice(2),rn="__reactFiber$"+yi,pn="__reactProps$"+yi,Yi="__reactContainer$"+yi,Ea="__reactEvents$"+yi,tl="__reactListeners$"+yi,nl="__reactHandles$"+yi,il="__reactResources$"+yi,os="__reactMarker$"+yi;function qr(t){delete t[rn],delete t[pn],delete t[Ea],delete t[tl],delete t[nl]}function Ta(t){var n=t[rn];if(n)return n;for(var a=t.parentNode;a;){if(n=a[Yi]||a[rn]){if(a=n.alternate,n.child!==null||a!==null&&a.child!==null)for(t=h0(t);t!==null;){if(a=t[rn])return a;t=h0(t)}return n}t=a,a=t.parentNode}return null}function Aa(t){if(t=t[rn]||t[Yi]){var n=t.tag;if(n===5||n===6||n===13||n===31||n===26||n===27||n===3)return t}return null}function ls(t){var n=t.tag;if(n===5||n===26||n===27||n===6)return t.stateNode;throw Error(s(33))}function R(t){var n=t[il];return n||(n=t[il]={hoistableStyles:new Map,hoistableScripts:new Map}),n}function j(t){t[os]=!0}var re=new Set,ne={};function Q(t,n){De(t,n),De(t+"Capture",n)}function De(t,n){for(ne[t]=n,t=0;t<n.length;t++)re.add(n[t])}var ze=RegExp("^[:A-Z_a-z\\u00C0-\\u00D6\\u00D8-\\u00F6\\u00F8-\\u02FF\\u0370-\\u037D\\u037F-\\u1FFF\\u200C-\\u200D\\u2070-\\u218F\\u2C00-\\u2FEF\\u3001-\\uD7FF\\uF900-\\uFDCF\\uFDF0-\\uFFFD][:A-Z_a-z\\u00C0-\\u00D6\\u00D8-\\u00F6\\u00F8-\\u02FF\\u0370-\\u037D\\u037F-\\u1FFF\\u200C-\\u200D\\u2070-\\u218F\\u2C00-\\u2FEF\\u3001-\\uD7FF\\uF900-\\uFDCF\\uFDF0-\\uFFFD\\-.0-9\\u00B7\\u0300-\\u036F\\u203F-\\u2040]*$"),Ne={},je={};function Ye(t){return qt.call(je,t)?!0:qt.call(Ne,t)?!1:ze.test(t)?je[t]=!0:(Ne[t]=!0,!1)}function tt(t,n,a){if(Ye(n))if(a===null)t.removeAttribute(n);else{switch(typeof a){case"undefined":case"function":case"symbol":t.removeAttribute(n);return;case"boolean":var r=n.toLowerCase().slice(0,5);if(r!=="data-"&&r!=="aria-"){t.removeAttribute(n);return}}t.setAttribute(n,""+a)}}function st(t,n,a){if(a===null)t.removeAttribute(n);else{switch(typeof a){case"undefined":case"function":case"symbol":case"boolean":t.removeAttribute(n);return}t.setAttribute(n,""+a)}}function He(t,n,a,r){if(r===null)t.removeAttribute(a);else{switch(typeof r){case"undefined":case"function":case"symbol":case"boolean":t.removeAttribute(a);return}t.setAttributeNS(n,a,""+r)}}function ft(t){switch(typeof t){case"bigint":case"boolean":case"number":case"string":case"undefined":return t;case"object":return t;default:return""}}function Yt(t){var n=t.type;return(t=t.nodeName)&&t.toLowerCase()==="input"&&(n==="checkbox"||n==="radio")}function Zt(t,n,a){var r=Object.getOwnPropertyDescriptor(t.constructor.prototype,n);if(!t.hasOwnProperty(n)&&typeof r<"u"&&typeof r.get=="function"&&typeof r.set=="function"){var u=r.get,f=r.set;return Object.defineProperty(t,n,{configurable:!0,get:function(){return u.call(this)},set:function(_){a=""+_,f.call(this,_)}}),Object.defineProperty(t,n,{enumerable:r.enumerable}),{getValue:function(){return a},setValue:function(_){a=""+_},stopTracking:function(){t._valueTracker=null,delete t[n]}}}}function Rt(t){if(!t._valueTracker){var n=Yt(t)?"checked":"value";t._valueTracker=Zt(t,n,""+t[n])}}function mn(t){if(!t)return!1;var n=t._valueTracker;if(!n)return!0;var a=n.getValue(),r="";return t&&(r=Yt(t)?t.checked?"true":"false":t.value),t=r,t!==a?(n.setValue(t),!0):!1}function Ve(t){if(t=t||(typeof document<"u"?document:void 0),typeof t>"u")return null;try{return t.activeElement||t.body}catch{return t.body}}var Un=/[\n"\\]/g;function it(t){return t.replace(Un,function(n){return"\\"+n.charCodeAt(0).toString(16)+" "})}function Ln(t,n,a,r,u,f,_,A){t.name="",_!=null&&typeof _!="function"&&typeof _!="symbol"&&typeof _!="boolean"?t.type=_:t.removeAttribute("type"),n!=null?_==="number"?(n===0&&t.value===""||t.value!=n)&&(t.value=""+ft(n)):t.value!==""+ft(n)&&(t.value=""+ft(n)):_!=="submit"&&_!=="reset"||t.removeAttribute("value"),n!=null?Si(t,_,ft(n)):a!=null?Si(t,_,ft(a)):r!=null&&t.removeAttribute("value"),u==null&&f!=null&&(t.defaultChecked=!!f),u!=null&&(t.checked=u&&typeof u!="function"&&typeof u!="symbol"),A!=null&&typeof A!="function"&&typeof A!="symbol"&&typeof A!="boolean"?t.name=""+ft(A):t.removeAttribute("name")}function Yn(t,n,a,r,u,f,_,A){if(f!=null&&typeof f!="function"&&typeof f!="symbol"&&typeof f!="boolean"&&(t.type=f),n!=null||a!=null){if(!(f!=="submit"&&f!=="reset"||n!=null)){Rt(t);return}a=a!=null?""+ft(a):"",n=n!=null?""+ft(n):a,A||n===t.value||(t.value=n),t.defaultValue=n}r=r??u,r=typeof r!="function"&&typeof r!="symbol"&&!!r,t.checked=A?t.checked:!!r,t.defaultChecked=!!r,_!=null&&typeof _!="function"&&typeof _!="symbol"&&typeof _!="boolean"&&(t.name=_),Rt(t)}function Si(t,n,a){n==="number"&&Ve(t.ownerDocument)===t||t.defaultValue===""+a||(t.defaultValue=""+a)}function Zn(t,n,a,r){if(t=t.options,n){n={};for(var u=0;u<a.length;u++)n["$"+a[u]]=!0;for(a=0;a<t.length;a++)u=n.hasOwnProperty("$"+t[a].value),t[a].selected!==u&&(t[a].selected=u),u&&r&&(t[a].defaultSelected=!0)}else{for(a=""+ft(a),n=null,u=0;u<t.length;u++){if(t[u].value===a){t[u].selected=!0,r&&(t[u].defaultSelected=!0);return}n!==null||t[u].disabled||(n=t[u])}n!==null&&(n.selected=!0)}}function Ot(t,n,a){if(n!=null&&(n=""+ft(n),n!==t.value&&(t.value=n),a==null)){t.defaultValue!==n&&(t.defaultValue=n);return}t.defaultValue=a!=null?""+ft(a):""}function on(t,n,a,r){if(n==null){if(r!=null){if(a!=null)throw Error(s(92));if(ee(r)){if(1<r.length)throw Error(s(93));r=r[0]}a=r}a==null&&(a=""),n=a}a=ft(n),t.defaultValue=a,r=t.textContent,r===a&&r!==""&&r!==null&&(t.value=r),Rt(t)}function On(t,n){if(n){var a=t.firstChild;if(a&&a===t.lastChild&&a.nodeType===3){a.nodeValue=n;return}}t.textContent=n}var ln=new Set("animationIterationCount aspectRatio borderImageOutset borderImageSlice borderImageWidth boxFlex boxFlexGroup boxOrdinalGroup columnCount columns flex flexGrow flexPositive flexShrink flexNegative flexOrder gridArea gridRow gridRowEnd gridRowSpan gridRowStart gridColumn gridColumnEnd gridColumnSpan gridColumnStart fontWeight lineClamp lineHeight opacity order orphans scale tabSize widows zIndex zoom fillOpacity floodOpacity stopOpacity strokeDasharray strokeDashoffset strokeMiterlimit strokeOpacity strokeWidth MozAnimationIterationCount MozBoxFlex MozBoxFlexGroup MozLineClamp msAnimationIterationCount msFlex msZoom msFlexGrow msFlexNegative msFlexOrder msFlexPositive msFlexShrink msGridColumn msGridColumnSpan msGridRow msGridRowSpan WebkitAnimationIterationCount WebkitBoxFlex WebKitBoxFlexGroup WebkitBoxOrdinalGroup WebkitColumnCount WebkitColumns WebkitFlex WebkitFlexGrow WebkitFlexPositive WebkitFlexShrink WebkitLineClamp".split(" "));function bi(t,n,a){var r=n.indexOf("--")===0;a==null||typeof a=="boolean"||a===""?r?t.setProperty(n,""):n==="float"?t.cssFloat="":t[n]="":r?t.setProperty(n,a):typeof a!="number"||a===0||ln.has(n)?n==="float"?t.cssFloat=a:t[n]=(""+a).trim():t[n]=a+"px"}function Zi(t,n,a){if(n!=null&&typeof n!="object")throw Error(s(62));if(t=t.style,a!=null){for(var r in a)!a.hasOwnProperty(r)||n!=null&&n.hasOwnProperty(r)||(r.indexOf("--")===0?t.setProperty(r,""):r==="float"?t.cssFloat="":t[r]="");for(var u in n)r=n[u],n.hasOwnProperty(u)&&a[u]!==r&&bi(t,u,r)}else for(var f in n)n.hasOwnProperty(f)&&bi(t,f,n[f])}function Vs(t){if(t.indexOf("-")===-1)return!1;switch(t){case"annotation-xml":case"color-profile":case"font-face":case"font-face-src":case"font-face-uri":case"font-face-format":case"font-face-name":case"missing-glyph":return!1;default:return!0}}var Iv=new Map([["acceptCharset","accept-charset"],["htmlFor","for"],["httpEquiv","http-equiv"],["crossOrigin","crossorigin"],["accentHeight","accent-height"],["alignmentBaseline","alignment-baseline"],["arabicForm","arabic-form"],["baselineShift","baseline-shift"],["capHeight","cap-height"],["clipPath","clip-path"],["clipRule","clip-rule"],["colorInterpolation","color-interpolation"],["colorInterpolationFilters","color-interpolation-filters"],["colorProfile","color-profile"],["colorRendering","color-rendering"],["dominantBaseline","dominant-baseline"],["enableBackground","enable-background"],["fillOpacity","fill-opacity"],["fillRule","fill-rule"],["floodColor","flood-color"],["floodOpacity","flood-opacity"],["fontFamily","font-family"],["fontSize","font-size"],["fontSizeAdjust","font-size-adjust"],["fontStretch","font-stretch"],["fontStyle","font-style"],["fontVariant","font-variant"],["fontWeight","font-weight"],["glyphName","glyph-name"],["glyphOrientationHorizontal","glyph-orientation-horizontal"],["glyphOrientationVertical","glyph-orientation-vertical"],["horizAdvX","horiz-adv-x"],["horizOriginX","horiz-origin-x"],["imageRendering","image-rendering"],["letterSpacing","letter-spacing"],["lightingColor","lighting-color"],["markerEnd","marker-end"],["markerMid","marker-mid"],["markerStart","marker-start"],["overlinePosition","overline-position"],["overlineThickness","overline-thickness"],["paintOrder","paint-order"],["panose-1","panose-1"],["pointerEvents","pointer-events"],["renderingIntent","rendering-intent"],["shapeRendering","shape-rendering"],["stopColor","stop-color"],["stopOpacity","stop-opacity"],["strikethroughPosition","strikethrough-position"],["strikethroughThickness","strikethrough-thickness"],["strokeDasharray","stroke-dasharray"],["strokeDashoffset","stroke-dashoffset"],["strokeLinecap","stroke-linecap"],["strokeLinejoin","stroke-linejoin"],["strokeMiterlimit","stroke-miterlimit"],["strokeOpacity","stroke-opacity"],["strokeWidth","stroke-width"],["textAnchor","text-anchor"],["textDecoration","text-decoration"],["textRendering","text-rendering"],["transformOrigin","transform-origin"],["underlinePosition","underline-position"],["underlineThickness","underline-thickness"],["unicodeBidi","unicode-bidi"],["unicodeRange","unicode-range"],["unitsPerEm","units-per-em"],["vAlphabetic","v-alphabetic"],["vHanging","v-hanging"],["vIdeographic","v-ideographic"],["vMathematical","v-mathematical"],["vectorEffect","vector-effect"],["vertAdvY","vert-adv-y"],["vertOriginX","vert-origin-x"],["vertOriginY","vert-origin-y"],["wordSpacing","word-spacing"],["writingMode","writing-mode"],["xmlnsXlink","xmlns:xlink"],["xHeight","x-height"]]),Fv=/^[\u0000-\u001F ]*j[\r\n\t]*a[\r\n\t]*v[\r\n\t]*a[\r\n\t]*s[\r\n\t]*c[\r\n\t]*r[\r\n\t]*i[\r\n\t]*p[\r\n\t]*t[\r\n\t]*:/i;function al(t){return Fv.test(""+t)?"javascript:throw new Error('React has blocked a javascript: URL as a security precaution.')":t}function Ki(){}var iu=null;function au(t){return t=t.target||t.srcElement||window,t.correspondingUseElement&&(t=t.correspondingUseElement),t.nodeType===3?t.parentNode:t}var ks=null,Xs=null;function ap(t){var n=Aa(t);if(n&&(t=n.stateNode)){var a=t[pn]||null;e:switch(t=n.stateNode,n.type){case"input":if(Ln(t,a.value,a.defaultValue,a.defaultValue,a.checked,a.defaultChecked,a.type,a.name),n=a.name,a.type==="radio"&&n!=null){for(a=t;a.parentNode;)a=a.parentNode;for(a=a.querySelectorAll('input[name="'+it(""+n)+'"][type="radio"]'),n=0;n<a.length;n++){var r=a[n];if(r!==t&&r.form===t.form){var u=r[pn]||null;if(!u)throw Error(s(90));Ln(r,u.value,u.defaultValue,u.defaultValue,u.checked,u.defaultChecked,u.type,u.name)}}for(n=0;n<a.length;n++)r=a[n],r.form===t.form&&mn(r)}break e;case"textarea":Ot(t,a.value,a.defaultValue);break e;case"select":n=a.value,n!=null&&Zn(t,!!a.multiple,n,!1)}}}var su=!1;function sp(t,n,a){if(su)return t(n,a);su=!0;try{var r=t(n);return r}finally{if(su=!1,(ks!==null||Xs!==null)&&(jl(),ks&&(n=ks,t=Xs,Xs=ks=null,ap(n),t)))for(n=0;n<t.length;n++)ap(t[n])}}function Yr(t,n){var a=t.stateNode;if(a===null)return null;var r=a[pn]||null;if(r===null)return null;a=r[n];e:switch(n){case"onClick":case"onClickCapture":case"onDoubleClick":case"onDoubleClickCapture":case"onMouseDown":case"onMouseDownCapture":case"onMouseMove":case"onMouseMoveCapture":case"onMouseUp":case"onMouseUpCapture":case"onMouseEnter":(r=!r.disabled)||(t=t.type,r=!(t==="button"||t==="input"||t==="select"||t==="textarea")),t=!r;break e;default:t=!1}if(t)return null;if(a&&typeof a!="function")throw Error(s(231,n,typeof a));return a}var Qi=!(typeof window>"u"||typeof window.document>"u"||typeof window.document.createElement>"u"),ru=!1;if(Qi)try{var Zr={};Object.defineProperty(Zr,"passive",{get:function(){ru=!0}}),window.addEventListener("test",Zr,Zr),window.removeEventListener("test",Zr,Zr)}catch{ru=!1}var Ra=null,ou=null,sl=null;function rp(){if(sl)return sl;var t,n=ou,a=n.length,r,u="value"in Ra?Ra.value:Ra.textContent,f=u.length;for(t=0;t<a&&n[t]===u[t];t++);var _=a-t;for(r=1;r<=_&&n[a-r]===u[f-r];r++);return sl=u.slice(t,1<r?1-r:void 0)}function rl(t){var n=t.keyCode;return"charCode"in t?(t=t.charCode,t===0&&n===13&&(t=13)):t=n,t===10&&(t=13),32<=t||t===13?t:0}function ol(){return!0}function op(){return!1}function Bn(t){function n(a,r,u,f,_){this._reactName=a,this._targetInst=u,this.type=r,this.nativeEvent=f,this.target=_,this.currentTarget=null;for(var A in t)t.hasOwnProperty(A)&&(a=t[A],this[A]=a?a(f):f[A]);return this.isDefaultPrevented=(f.defaultPrevented!=null?f.defaultPrevented:f.returnValue===!1)?ol:op,this.isPropagationStopped=op,this}return y(n.prototype,{preventDefault:function(){this.defaultPrevented=!0;var a=this.nativeEvent;a&&(a.preventDefault?a.preventDefault():typeof a.returnValue!="unknown"&&(a.returnValue=!1),this.isDefaultPrevented=ol)},stopPropagation:function(){var a=this.nativeEvent;a&&(a.stopPropagation?a.stopPropagation():typeof a.cancelBubble!="unknown"&&(a.cancelBubble=!0),this.isPropagationStopped=ol)},persist:function(){},isPersistent:ol}),n}var cs={eventPhase:0,bubbles:0,cancelable:0,timeStamp:function(t){return t.timeStamp||Date.now()},defaultPrevented:0,isTrusted:0},ll=Bn(cs),Kr=y({},cs,{view:0,detail:0}),zv=Bn(Kr),lu,cu,Qr,cl=y({},Kr,{screenX:0,screenY:0,clientX:0,clientY:0,pageX:0,pageY:0,ctrlKey:0,shiftKey:0,altKey:0,metaKey:0,getModifierState:fu,button:0,buttons:0,relatedTarget:function(t){return t.relatedTarget===void 0?t.fromElement===t.srcElement?t.toElement:t.fromElement:t.relatedTarget},movementX:function(t){return"movementX"in t?t.movementX:(t!==Qr&&(Qr&&t.type==="mousemove"?(lu=t.screenX-Qr.screenX,cu=t.screenY-Qr.screenY):cu=lu=0,Qr=t),lu)},movementY:function(t){return"movementY"in t?t.movementY:cu}}),lp=Bn(cl),Bv=y({},cl,{dataTransfer:0}),Hv=Bn(Bv),Gv=y({},Kr,{relatedTarget:0}),uu=Bn(Gv),Vv=y({},cs,{animationName:0,elapsedTime:0,pseudoElement:0}),kv=Bn(Vv),Xv=y({},cs,{clipboardData:function(t){return"clipboardData"in t?t.clipboardData:window.clipboardData}}),jv=Bn(Xv),Wv=y({},cs,{data:0}),cp=Bn(Wv),qv={Esc:"Escape",Spacebar:" ",Left:"ArrowLeft",Up:"ArrowUp",Right:"ArrowRight",Down:"ArrowDown",Del:"Delete",Win:"OS",Menu:"ContextMenu",Apps:"ContextMenu",Scroll:"ScrollLock",MozPrintableKey:"Unidentified"},Yv={8:"Backspace",9:"Tab",12:"Clear",13:"Enter",16:"Shift",17:"Control",18:"Alt",19:"Pause",20:"CapsLock",27:"Escape",32:" ",33:"PageUp",34:"PageDown",35:"End",36:"Home",37:"ArrowLeft",38:"ArrowUp",39:"ArrowRight",40:"ArrowDown",45:"Insert",46:"Delete",112:"F1",113:"F2",114:"F3",115:"F4",116:"F5",117:"F6",118:"F7",119:"F8",120:"F9",121:"F10",122:"F11",123:"F12",144:"NumLock",145:"ScrollLock",224:"Meta"},Zv={Alt:"altKey",Control:"ctrlKey",Meta:"metaKey",Shift:"shiftKey"};function Kv(t){var n=this.nativeEvent;return n.getModifierState?n.getModifierState(t):(t=Zv[t])?!!n[t]:!1}function fu(){return Kv}var Qv=y({},Kr,{key:function(t){if(t.key){var n=qv[t.key]||t.key;if(n!=="Unidentified")return n}return t.type==="keypress"?(t=rl(t),t===13?"Enter":String.fromCharCode(t)):t.type==="keydown"||t.type==="keyup"?Yv[t.keyCode]||"Unidentified":""},code:0,location:0,ctrlKey:0,shiftKey:0,altKey:0,metaKey:0,repeat:0,locale:0,getModifierState:fu,charCode:function(t){return t.type==="keypress"?rl(t):0},keyCode:function(t){return t.type==="keydown"||t.type==="keyup"?t.keyCode:0},which:function(t){return t.type==="keypress"?rl(t):t.type==="keydown"||t.type==="keyup"?t.keyCode:0}}),Jv=Bn(Qv),$v=y({},cl,{pointerId:0,width:0,height:0,pressure:0,tangentialPressure:0,tiltX:0,tiltY:0,twist:0,pointerType:0,isPrimary:0}),up=Bn($v),ex=y({},Kr,{touches:0,targetTouches:0,changedTouches:0,altKey:0,metaKey:0,ctrlKey:0,shiftKey:0,getModifierState:fu}),tx=Bn(ex),nx=y({},cs,{propertyName:0,elapsedTime:0,pseudoElement:0}),ix=Bn(nx),ax=y({},cl,{deltaX:function(t){return"deltaX"in t?t.deltaX:"wheelDeltaX"in t?-t.wheelDeltaX:0},deltaY:function(t){return"deltaY"in t?t.deltaY:"wheelDeltaY"in t?-t.wheelDeltaY:"wheelDelta"in t?-t.wheelDelta:0},deltaZ:0,deltaMode:0}),sx=Bn(ax),rx=y({},cs,{newState:0,oldState:0}),ox=Bn(rx),lx=[9,13,27,32],du=Qi&&"CompositionEvent"in window,Jr=null;Qi&&"documentMode"in document&&(Jr=document.documentMode);var cx=Qi&&"TextEvent"in window&&!Jr,fp=Qi&&(!du||Jr&&8<Jr&&11>=Jr),dp=" ",hp=!1;function pp(t,n){switch(t){case"keyup":return lx.indexOf(n.keyCode)!==-1;case"keydown":return n.keyCode!==229;case"keypress":case"mousedown":case"focusout":return!0;default:return!1}}function mp(t){return t=t.detail,typeof t=="object"&&"data"in t?t.data:null}var js=!1;function ux(t,n){switch(t){case"compositionend":return mp(n);case"keypress":return n.which!==32?null:(hp=!0,dp);case"textInput":return t=n.data,t===dp&&hp?null:t;default:return null}}function fx(t,n){if(js)return t==="compositionend"||!du&&pp(t,n)?(t=rp(),sl=ou=Ra=null,js=!1,t):null;switch(t){case"paste":return null;case"keypress":if(!(n.ctrlKey||n.altKey||n.metaKey)||n.ctrlKey&&n.altKey){if(n.char&&1<n.char.length)return n.char;if(n.which)return String.fromCharCode(n.which)}return null;case"compositionend":return fp&&n.locale!=="ko"?null:n.data;default:return null}}var dx={color:!0,date:!0,datetime:!0,"datetime-local":!0,email:!0,month:!0,number:!0,password:!0,range:!0,search:!0,tel:!0,text:!0,time:!0,url:!0,week:!0};function gp(t){var n=t&&t.nodeName&&t.nodeName.toLowerCase();return n==="input"?!!dx[t.type]:n==="textarea"}function _p(t,n,a,r){ks?Xs?Xs.push(r):Xs=[r]:ks=r,n=Jl(n,"onChange"),0<n.length&&(a=new ll("onChange","change",null,a,r),t.push({event:a,listeners:n}))}var $r=null,eo=null;function hx(t){$g(t,0)}function ul(t){var n=ls(t);if(mn(n))return t}function vp(t,n){if(t==="change")return n}var xp=!1;if(Qi){var hu;if(Qi){var pu="oninput"in document;if(!pu){var yp=document.createElement("div");yp.setAttribute("oninput","return;"),pu=typeof yp.oninput=="function"}hu=pu}else hu=!1;xp=hu&&(!document.documentMode||9<document.documentMode)}function Sp(){$r&&($r.detachEvent("onpropertychange",bp),eo=$r=null)}function bp(t){if(t.propertyName==="value"&&ul(eo)){var n=[];_p(n,eo,t,au(t)),sp(hx,n)}}function px(t,n,a){t==="focusin"?(Sp(),$r=n,eo=a,$r.attachEvent("onpropertychange",bp)):t==="focusout"&&Sp()}function mx(t){if(t==="selectionchange"||t==="keyup"||t==="keydown")return ul(eo)}function gx(t,n){if(t==="click")return ul(n)}function _x(t,n){if(t==="input"||t==="change")return ul(n)}function vx(t,n){return t===n&&(t!==0||1/t===1/n)||t!==t&&n!==n}var Kn=typeof Object.is=="function"?Object.is:vx;function to(t,n){if(Kn(t,n))return!0;if(typeof t!="object"||t===null||typeof n!="object"||n===null)return!1;var a=Object.keys(t),r=Object.keys(n);if(a.length!==r.length)return!1;for(r=0;r<a.length;r++){var u=a[r];if(!qt.call(n,u)||!Kn(t[u],n[u]))return!1}return!0}function Mp(t){for(;t&&t.firstChild;)t=t.firstChild;return t}function Ep(t,n){var a=Mp(t);t=0;for(var r;a;){if(a.nodeType===3){if(r=t+a.textContent.length,t<=n&&r>=n)return{node:a,offset:n-t};t=r}e:{for(;a;){if(a.nextSibling){a=a.nextSibling;break e}a=a.parentNode}a=void 0}a=Mp(a)}}function Tp(t,n){return t&&n?t===n?!0:t&&t.nodeType===3?!1:n&&n.nodeType===3?Tp(t,n.parentNode):"contains"in t?t.contains(n):t.compareDocumentPosition?!!(t.compareDocumentPosition(n)&16):!1:!1}function Ap(t){t=t!=null&&t.ownerDocument!=null&&t.ownerDocument.defaultView!=null?t.ownerDocument.defaultView:window;for(var n=Ve(t.document);n instanceof t.HTMLIFrameElement;){try{var a=typeof n.contentWindow.location.href=="string"}catch{a=!1}if(a)t=n.contentWindow;else break;n=Ve(t.document)}return n}function mu(t){var n=t&&t.nodeName&&t.nodeName.toLowerCase();return n&&(n==="input"&&(t.type==="text"||t.type==="search"||t.type==="tel"||t.type==="url"||t.type==="password")||n==="textarea"||t.contentEditable==="true")}var xx=Qi&&"documentMode"in document&&11>=document.documentMode,Ws=null,gu=null,no=null,_u=!1;function Rp(t,n,a){var r=a.window===a?a.document:a.nodeType===9?a:a.ownerDocument;_u||Ws==null||Ws!==Ve(r)||(r=Ws,"selectionStart"in r&&mu(r)?r={start:r.selectionStart,end:r.selectionEnd}:(r=(r.ownerDocument&&r.ownerDocument.defaultView||window).getSelection(),r={anchorNode:r.anchorNode,anchorOffset:r.anchorOffset,focusNode:r.focusNode,focusOffset:r.focusOffset}),no&&to(no,r)||(no=r,r=Jl(gu,"onSelect"),0<r.length&&(n=new ll("onSelect","select",null,n,a),t.push({event:n,listeners:r}),n.target=Ws)))}function us(t,n){var a={};return a[t.toLowerCase()]=n.toLowerCase(),a["Webkit"+t]="webkit"+n,a["Moz"+t]="moz"+n,a}var qs={animationend:us("Animation","AnimationEnd"),animationiteration:us("Animation","AnimationIteration"),animationstart:us("Animation","AnimationStart"),transitionrun:us("Transition","TransitionRun"),transitionstart:us("Transition","TransitionStart"),transitioncancel:us("Transition","TransitionCancel"),transitionend:us("Transition","TransitionEnd")},vu={},wp={};Qi&&(wp=document.createElement("div").style,"AnimationEvent"in window||(delete qs.animationend.animation,delete qs.animationiteration.animation,delete qs.animationstart.animation),"TransitionEvent"in window||delete qs.transitionend.transition);function fs(t){if(vu[t])return vu[t];if(!qs[t])return t;var n=qs[t],a;for(a in n)if(n.hasOwnProperty(a)&&a in wp)return vu[t]=n[a];return t}var Cp=fs("animationend"),Dp=fs("animationiteration"),Np=fs("animationstart"),yx=fs("transitionrun"),Sx=fs("transitionstart"),bx=fs("transitioncancel"),Up=fs("transitionend"),Lp=new Map,xu="abort auxClick beforeToggle cancel canPlay canPlayThrough click close contextMenu copy cut drag dragEnd dragEnter dragExit dragLeave dragOver dragStart drop durationChange emptied encrypted ended error gotPointerCapture input invalid keyDown keyPress keyUp load loadedData loadedMetadata loadStart lostPointerCapture mouseDown mouseMove mouseOut mouseOver mouseUp paste pause play playing pointerCancel pointerDown pointerMove pointerOut pointerOver pointerUp progress rateChange reset resize seeked seeking stalled submit suspend timeUpdate touchCancel touchEnd touchStart volumeChange scroll toggle touchMove waiting wheel".split(" ");xu.push("scrollEnd");function Mi(t,n){Lp.set(t,n),Q(n,[t])}var fl=typeof reportError=="function"?reportError:function(t){if(typeof window=="object"&&typeof window.ErrorEvent=="function"){var n=new window.ErrorEvent("error",{bubbles:!0,cancelable:!0,message:typeof t=="object"&&t!==null&&typeof t.message=="string"?String(t.message):String(t),error:t});if(!window.dispatchEvent(n))return}else if(typeof process=="object"&&typeof process.emit=="function"){process.emit("uncaughtException",t);return}console.error(t)},oi=[],Ys=0,yu=0;function dl(){for(var t=Ys,n=yu=Ys=0;n<t;){var a=oi[n];oi[n++]=null;var r=oi[n];oi[n++]=null;var u=oi[n];oi[n++]=null;var f=oi[n];if(oi[n++]=null,r!==null&&u!==null){var _=r.pending;_===null?u.next=u:(u.next=_.next,_.next=u),r.pending=u}f!==0&&Op(a,u,f)}}function hl(t,n,a,r){oi[Ys++]=t,oi[Ys++]=n,oi[Ys++]=a,oi[Ys++]=r,yu|=r,t.lanes|=r,t=t.alternate,t!==null&&(t.lanes|=r)}function Su(t,n,a,r){return hl(t,n,a,r),pl(t)}function ds(t,n){return hl(t,null,null,n),pl(t)}function Op(t,n,a){t.lanes|=a;var r=t.alternate;r!==null&&(r.lanes|=a);for(var u=!1,f=t.return;f!==null;)f.childLanes|=a,r=f.alternate,r!==null&&(r.childLanes|=a),f.tag===22&&(t=f.stateNode,t===null||t._visibility&1||(u=!0)),t=f,f=f.return;return t.tag===3?(f=t.stateNode,u&&n!==null&&(u=31-Le(a),t=f.hiddenUpdates,r=t[u],r===null?t[u]=[n]:r.push(n),n.lane=a|536870912),f):null}function pl(t){if(50<To)throw To=0,Nf=null,Error(s(185));for(var n=t.return;n!==null;)t=n,n=t.return;return t.tag===3?t.stateNode:null}var Zs={};function Mx(t,n,a,r){this.tag=t,this.key=a,this.sibling=this.child=this.return=this.stateNode=this.type=this.elementType=null,this.index=0,this.refCleanup=this.ref=null,this.pendingProps=n,this.dependencies=this.memoizedState=this.updateQueue=this.memoizedProps=null,this.mode=r,this.subtreeFlags=this.flags=0,this.deletions=null,this.childLanes=this.lanes=0,this.alternate=null}function Qn(t,n,a,r){return new Mx(t,n,a,r)}function bu(t){return t=t.prototype,!(!t||!t.isReactComponent)}function Ji(t,n){var a=t.alternate;return a===null?(a=Qn(t.tag,n,t.key,t.mode),a.elementType=t.elementType,a.type=t.type,a.stateNode=t.stateNode,a.alternate=t,t.alternate=a):(a.pendingProps=n,a.type=t.type,a.flags=0,a.subtreeFlags=0,a.deletions=null),a.flags=t.flags&65011712,a.childLanes=t.childLanes,a.lanes=t.lanes,a.child=t.child,a.memoizedProps=t.memoizedProps,a.memoizedState=t.memoizedState,a.updateQueue=t.updateQueue,n=t.dependencies,a.dependencies=n===null?null:{lanes:n.lanes,firstContext:n.firstContext},a.sibling=t.sibling,a.index=t.index,a.ref=t.ref,a.refCleanup=t.refCleanup,a}function Pp(t,n){t.flags&=65011714;var a=t.alternate;return a===null?(t.childLanes=0,t.lanes=n,t.child=null,t.subtreeFlags=0,t.memoizedProps=null,t.memoizedState=null,t.updateQueue=null,t.dependencies=null,t.stateNode=null):(t.childLanes=a.childLanes,t.lanes=a.lanes,t.child=a.child,t.subtreeFlags=0,t.deletions=null,t.memoizedProps=a.memoizedProps,t.memoizedState=a.memoizedState,t.updateQueue=a.updateQueue,t.type=a.type,n=a.dependencies,t.dependencies=n===null?null:{lanes:n.lanes,firstContext:n.firstContext}),t}function ml(t,n,a,r,u,f){var _=0;if(r=t,typeof t=="function")bu(t)&&(_=1);else if(typeof t=="string")_=wy(t,a,Re.current)?26:t==="html"||t==="head"||t==="body"?27:5;else e:switch(t){case D:return t=Qn(31,a,n,u),t.elementType=D,t.lanes=f,t;case w:return hs(a.children,u,f,n);case b:_=8,u|=24;break;case S:return t=Qn(12,a,n,u|2),t.elementType=S,t.lanes=f,t;case V:return t=Qn(13,a,n,u),t.elementType=V,t.lanes=f,t;case H:return t=Qn(19,a,n,u),t.elementType=H,t.lanes=f,t;default:if(typeof t=="object"&&t!==null)switch(t.$$typeof){case U:_=10;break e;case C:_=9;break e;case N:_=11;break e;case F:_=14;break e;case T:_=16,r=null;break e}_=29,a=Error(s(130,t===null?"null":typeof t,"")),r=null}return n=Qn(_,a,n,u),n.elementType=t,n.type=r,n.lanes=f,n}function hs(t,n,a,r){return t=Qn(7,t,r,n),t.lanes=a,t}function Mu(t,n,a){return t=Qn(6,t,null,n),t.lanes=a,t}function Ip(t){var n=Qn(18,null,null,0);return n.stateNode=t,n}function Eu(t,n,a){return n=Qn(4,t.children!==null?t.children:[],t.key,n),n.lanes=a,n.stateNode={containerInfo:t.containerInfo,pendingChildren:null,implementation:t.implementation},n}var Fp=new WeakMap;function li(t,n){if(typeof t=="object"&&t!==null){var a=Fp.get(t);return a!==void 0?a:(n={value:t,source:n,stack:k(n)},Fp.set(t,n),n)}return{value:t,source:n,stack:k(n)}}var Ks=[],Qs=0,gl=null,io=0,ci=[],ui=0,wa=null,Li=1,Oi="";function $i(t,n){Ks[Qs++]=io,Ks[Qs++]=gl,gl=t,io=n}function zp(t,n,a){ci[ui++]=Li,ci[ui++]=Oi,ci[ui++]=wa,wa=t;var r=Li;t=Oi;var u=32-Le(r)-1;r&=~(1<<u),a+=1;var f=32-Le(n)+u;if(30<f){var _=u-u%5;f=(r&(1<<_)-1).toString(32),r>>=_,u-=_,Li=1<<32-Le(n)+u|a<<u|r,Oi=f+t}else Li=1<<f|a<<u|r,Oi=t}function Tu(t){t.return!==null&&($i(t,1),zp(t,1,0))}function Au(t){for(;t===gl;)gl=Ks[--Qs],Ks[Qs]=null,io=Ks[--Qs],Ks[Qs]=null;for(;t===wa;)wa=ci[--ui],ci[ui]=null,Oi=ci[--ui],ci[ui]=null,Li=ci[--ui],ci[ui]=null}function Bp(t,n){ci[ui++]=Li,ci[ui++]=Oi,ci[ui++]=wa,Li=n.id,Oi=n.overflow,wa=t}var bn=null,Xt=null,bt=!1,Ca=null,fi=!1,Ru=Error(s(519));function Da(t){var n=Error(s(418,1<arguments.length&&arguments[1]!==void 0&&arguments[1]?"text":"HTML",""));throw ao(li(n,t)),Ru}function Hp(t){var n=t.stateNode,a=t.type,r=t.memoizedProps;switch(n[rn]=t,n[pn]=r,a){case"dialog":_t("cancel",n),_t("close",n);break;case"iframe":case"object":case"embed":_t("load",n);break;case"video":case"audio":for(a=0;a<Ro.length;a++)_t(Ro[a],n);break;case"source":_t("error",n);break;case"img":case"image":case"link":_t("error",n),_t("load",n);break;case"details":_t("toggle",n);break;case"input":_t("invalid",n),Yn(n,r.value,r.defaultValue,r.checked,r.defaultChecked,r.type,r.name,!0);break;case"select":_t("invalid",n);break;case"textarea":_t("invalid",n),on(n,r.value,r.defaultValue,r.children)}a=r.children,typeof a!="string"&&typeof a!="number"&&typeof a!="bigint"||n.textContent===""+a||r.suppressHydrationWarning===!0||i0(n.textContent,a)?(r.popover!=null&&(_t("beforetoggle",n),_t("toggle",n)),r.onScroll!=null&&_t("scroll",n),r.onScrollEnd!=null&&_t("scrollend",n),r.onClick!=null&&(n.onclick=Ki),n=!0):n=!1,n||Da(t,!0)}function Gp(t){for(bn=t.return;bn;)switch(bn.tag){case 5:case 31:case 13:fi=!1;return;case 27:case 3:fi=!0;return;default:bn=bn.return}}function Js(t){if(t!==bn)return!1;if(!bt)return Gp(t),bt=!0,!1;var n=t.tag,a;if((a=n!==3&&n!==27)&&((a=n===5)&&(a=t.type,a=!(a!=="form"&&a!=="button")||Wf(t.type,t.memoizedProps)),a=!a),a&&Xt&&Da(t),Gp(t),n===13){if(t=t.memoizedState,t=t!==null?t.dehydrated:null,!t)throw Error(s(317));Xt=d0(t)}else if(n===31){if(t=t.memoizedState,t=t!==null?t.dehydrated:null,!t)throw Error(s(317));Xt=d0(t)}else n===27?(n=Xt,Xa(t.type)?(t=Qf,Qf=null,Xt=t):Xt=n):Xt=bn?hi(t.stateNode.nextSibling):null;return!0}function ps(){Xt=bn=null,bt=!1}function wu(){var t=Ca;return t!==null&&(kn===null?kn=t:kn.push.apply(kn,t),Ca=null),t}function ao(t){Ca===null?Ca=[t]:Ca.push(t)}var Cu=I(null),ms=null,ea=null;function Na(t,n,a){ve(Cu,n._currentValue),n._currentValue=a}function ta(t){t._currentValue=Cu.current,Y(Cu)}function Du(t,n,a){for(;t!==null;){var r=t.alternate;if((t.childLanes&n)!==n?(t.childLanes|=n,r!==null&&(r.childLanes|=n)):r!==null&&(r.childLanes&n)!==n&&(r.childLanes|=n),t===a)break;t=t.return}}function Nu(t,n,a,r){var u=t.child;for(u!==null&&(u.return=t);u!==null;){var f=u.dependencies;if(f!==null){var _=u.child;f=f.firstContext;e:for(;f!==null;){var A=f;f=u;for(var B=0;B<n.length;B++)if(A.context===n[B]){f.lanes|=a,A=f.alternate,A!==null&&(A.lanes|=a),Du(f.return,a,t),r||(_=null);break e}f=A.next}}else if(u.tag===18){if(_=u.return,_===null)throw Error(s(341));_.lanes|=a,f=_.alternate,f!==null&&(f.lanes|=a),Du(_,a,t),_=null}else _=u.child;if(_!==null)_.return=u;else for(_=u;_!==null;){if(_===t){_=null;break}if(u=_.sibling,u!==null){u.return=_.return,_=u;break}_=_.return}u=_}}function $s(t,n,a,r){t=null;for(var u=n,f=!1;u!==null;){if(!f){if((u.flags&524288)!==0)f=!0;else if((u.flags&262144)!==0)break}if(u.tag===10){var _=u.alternate;if(_===null)throw Error(s(387));if(_=_.memoizedProps,_!==null){var A=u.type;Kn(u.pendingProps.value,_.value)||(t!==null?t.push(A):t=[A])}}else if(u===xe.current){if(_=u.alternate,_===null)throw Error(s(387));_.memoizedState.memoizedState!==u.memoizedState.memoizedState&&(t!==null?t.push(Uo):t=[Uo])}u=u.return}t!==null&&Nu(n,t,a,r),n.flags|=262144}function _l(t){for(t=t.firstContext;t!==null;){if(!Kn(t.context._currentValue,t.memoizedValue))return!0;t=t.next}return!1}function gs(t){ms=t,ea=null,t=t.dependencies,t!==null&&(t.firstContext=null)}function Mn(t){return Vp(ms,t)}function vl(t,n){return ms===null&&gs(t),Vp(t,n)}function Vp(t,n){var a=n._currentValue;if(n={context:n,memoizedValue:a,next:null},ea===null){if(t===null)throw Error(s(308));ea=n,t.dependencies={lanes:0,firstContext:n},t.flags|=524288}else ea=ea.next=n;return a}var Ex=typeof AbortController<"u"?AbortController:function(){var t=[],n=this.signal={aborted:!1,addEventListener:function(a,r){t.push(r)}};this.abort=function(){n.aborted=!0,t.forEach(function(a){return a()})}},Tx=o.unstable_scheduleCallback,Ax=o.unstable_NormalPriority,cn={$$typeof:U,Consumer:null,Provider:null,_currentValue:null,_currentValue2:null,_threadCount:0};function Uu(){return{controller:new Ex,data:new Map,refCount:0}}function so(t){t.refCount--,t.refCount===0&&Tx(Ax,function(){t.controller.abort()})}var ro=null,Lu=0,er=0,tr=null;function Rx(t,n){if(ro===null){var a=ro=[];Lu=0,er=Ff(),tr={status:"pending",value:void 0,then:function(r){a.push(r)}}}return Lu++,n.then(kp,kp),n}function kp(){if(--Lu===0&&ro!==null){tr!==null&&(tr.status="fulfilled");var t=ro;ro=null,er=0,tr=null;for(var n=0;n<t.length;n++)(0,t[n])()}}function wx(t,n){var a=[],r={status:"pending",value:null,reason:null,then:function(u){a.push(u)}};return t.then(function(){r.status="fulfilled",r.value=n;for(var u=0;u<a.length;u++)(0,a[u])(n)},function(u){for(r.status="rejected",r.reason=u,u=0;u<a.length;u++)(0,a[u])(void 0)}),r}var Xp=P.S;P.S=function(t,n){Rg=M(),typeof n=="object"&&n!==null&&typeof n.then=="function"&&Rx(t,n),Xp!==null&&Xp(t,n)};var _s=I(null);function Ou(){var t=_s.current;return t!==null?t:kt.pooledCache}function xl(t,n){n===null?ve(_s,_s.current):ve(_s,n.pool)}function jp(){var t=Ou();return t===null?null:{parent:cn._currentValue,pool:t}}var nr=Error(s(460)),Pu=Error(s(474)),yl=Error(s(542)),Sl={then:function(){}};function Wp(t){return t=t.status,t==="fulfilled"||t==="rejected"}function qp(t,n,a){switch(a=t[a],a===void 0?t.push(n):a!==n&&(n.then(Ki,Ki),n=a),n.status){case"fulfilled":return n.value;case"rejected":throw t=n.reason,Zp(t),t;default:if(typeof n.status=="string")n.then(Ki,Ki);else{if(t=kt,t!==null&&100<t.shellSuspendCounter)throw Error(s(482));t=n,t.status="pending",t.then(function(r){if(n.status==="pending"){var u=n;u.status="fulfilled",u.value=r}},function(r){if(n.status==="pending"){var u=n;u.status="rejected",u.reason=r}})}switch(n.status){case"fulfilled":return n.value;case"rejected":throw t=n.reason,Zp(t),t}throw xs=n,nr}}function vs(t){try{var n=t._init;return n(t._payload)}catch(a){throw a!==null&&typeof a=="object"&&typeof a.then=="function"?(xs=a,nr):a}}var xs=null;function Yp(){if(xs===null)throw Error(s(459));var t=xs;return xs=null,t}function Zp(t){if(t===nr||t===yl)throw Error(s(483))}var ir=null,oo=0;function bl(t){var n=oo;return oo+=1,ir===null&&(ir=[]),qp(ir,t,n)}function lo(t,n){n=n.props.ref,t.ref=n!==void 0?n:null}function Ml(t,n){throw n.$$typeof===g?Error(s(525)):(t=Object.prototype.toString.call(n),Error(s(31,t==="[object Object]"?"object with keys {"+Object.keys(n).join(", ")+"}":t)))}function Kp(t){function n(Z,X){if(t){var J=Z.deletions;J===null?(Z.deletions=[X],Z.flags|=16):J.push(X)}}function a(Z,X){if(!t)return null;for(;X!==null;)n(Z,X),X=X.sibling;return null}function r(Z){for(var X=new Map;Z!==null;)Z.key!==null?X.set(Z.key,Z):X.set(Z.index,Z),Z=Z.sibling;return X}function u(Z,X){return Z=Ji(Z,X),Z.index=0,Z.sibling=null,Z}function f(Z,X,J){return Z.index=J,t?(J=Z.alternate,J!==null?(J=J.index,J<X?(Z.flags|=67108866,X):J):(Z.flags|=67108866,X)):(Z.flags|=1048576,X)}function _(Z){return t&&Z.alternate===null&&(Z.flags|=67108866),Z}function A(Z,X,J,ge){return X===null||X.tag!==6?(X=Mu(J,Z.mode,ge),X.return=Z,X):(X=u(X,J),X.return=Z,X)}function B(Z,X,J,ge){var Qe=J.type;return Qe===w?he(Z,X,J.props.children,ge,J.key):X!==null&&(X.elementType===Qe||typeof Qe=="object"&&Qe!==null&&Qe.$$typeof===T&&vs(Qe)===X.type)?(X=u(X,J.props),lo(X,J),X.return=Z,X):(X=ml(J.type,J.key,J.props,null,Z.mode,ge),lo(X,J),X.return=Z,X)}function $(Z,X,J,ge){return X===null||X.tag!==4||X.stateNode.containerInfo!==J.containerInfo||X.stateNode.implementation!==J.implementation?(X=Eu(J,Z.mode,ge),X.return=Z,X):(X=u(X,J.children||[]),X.return=Z,X)}function he(Z,X,J,ge,Qe){return X===null||X.tag!==7?(X=hs(J,Z.mode,ge,Qe),X.return=Z,X):(X=u(X,J),X.return=Z,X)}function _e(Z,X,J){if(typeof X=="string"&&X!==""||typeof X=="number"||typeof X=="bigint")return X=Mu(""+X,Z.mode,J),X.return=Z,X;if(typeof X=="object"&&X!==null){switch(X.$$typeof){case x:return J=ml(X.type,X.key,X.props,null,Z.mode,J),lo(J,X),J.return=Z,J;case E:return X=Eu(X,Z.mode,J),X.return=Z,X;case T:return X=vs(X),_e(Z,X,J)}if(ee(X)||te(X))return X=hs(X,Z.mode,J,null),X.return=Z,X;if(typeof X.then=="function")return _e(Z,bl(X),J);if(X.$$typeof===U)return _e(Z,vl(Z,X),J);Ml(Z,X)}return null}function ae(Z,X,J,ge){var Qe=X!==null?X.key:null;if(typeof J=="string"&&J!==""||typeof J=="number"||typeof J=="bigint")return Qe!==null?null:A(Z,X,""+J,ge);if(typeof J=="object"&&J!==null){switch(J.$$typeof){case x:return J.key===Qe?B(Z,X,J,ge):null;case E:return J.key===Qe?$(Z,X,J,ge):null;case T:return J=vs(J),ae(Z,X,J,ge)}if(ee(J)||te(J))return Qe!==null?null:he(Z,X,J,ge,null);if(typeof J.then=="function")return ae(Z,X,bl(J),ge);if(J.$$typeof===U)return ae(Z,X,vl(Z,J),ge);Ml(Z,J)}return null}function oe(Z,X,J,ge,Qe){if(typeof ge=="string"&&ge!==""||typeof ge=="number"||typeof ge=="bigint")return Z=Z.get(J)||null,A(X,Z,""+ge,Qe);if(typeof ge=="object"&&ge!==null){switch(ge.$$typeof){case x:return Z=Z.get(ge.key===null?J:ge.key)||null,B(X,Z,ge,Qe);case E:return Z=Z.get(ge.key===null?J:ge.key)||null,$(X,Z,ge,Qe);case T:return ge=vs(ge),oe(Z,X,J,ge,Qe)}if(ee(ge)||te(ge))return Z=Z.get(J)||null,he(X,Z,ge,Qe,null);if(typeof ge.then=="function")return oe(Z,X,J,bl(ge),Qe);if(ge.$$typeof===U)return oe(Z,X,J,vl(X,ge),Qe);Ml(X,ge)}return null}function Ge(Z,X,J,ge){for(var Qe=null,wt=null,qe=X,dt=X=0,St=null;qe!==null&&dt<J.length;dt++){qe.index>dt?(St=qe,qe=null):St=qe.sibling;var Ct=ae(Z,qe,J[dt],ge);if(Ct===null){qe===null&&(qe=St);break}t&&qe&&Ct.alternate===null&&n(Z,qe),X=f(Ct,X,dt),wt===null?Qe=Ct:wt.sibling=Ct,wt=Ct,qe=St}if(dt===J.length)return a(Z,qe),bt&&$i(Z,dt),Qe;if(qe===null){for(;dt<J.length;dt++)qe=_e(Z,J[dt],ge),qe!==null&&(X=f(qe,X,dt),wt===null?Qe=qe:wt.sibling=qe,wt=qe);return bt&&$i(Z,dt),Qe}for(qe=r(qe);dt<J.length;dt++)St=oe(qe,Z,dt,J[dt],ge),St!==null&&(t&&St.alternate!==null&&qe.delete(St.key===null?dt:St.key),X=f(St,X,dt),wt===null?Qe=St:wt.sibling=St,wt=St);return t&&qe.forEach(function(Za){return n(Z,Za)}),bt&&$i(Z,dt),Qe}function Je(Z,X,J,ge){if(J==null)throw Error(s(151));for(var Qe=null,wt=null,qe=X,dt=X=0,St=null,Ct=J.next();qe!==null&&!Ct.done;dt++,Ct=J.next()){qe.index>dt?(St=qe,qe=null):St=qe.sibling;var Za=ae(Z,qe,Ct.value,ge);if(Za===null){qe===null&&(qe=St);break}t&&qe&&Za.alternate===null&&n(Z,qe),X=f(Za,X,dt),wt===null?Qe=Za:wt.sibling=Za,wt=Za,qe=St}if(Ct.done)return a(Z,qe),bt&&$i(Z,dt),Qe;if(qe===null){for(;!Ct.done;dt++,Ct=J.next())Ct=_e(Z,Ct.value,ge),Ct!==null&&(X=f(Ct,X,dt),wt===null?Qe=Ct:wt.sibling=Ct,wt=Ct);return bt&&$i(Z,dt),Qe}for(qe=r(qe);!Ct.done;dt++,Ct=J.next())Ct=oe(qe,Z,dt,Ct.value,ge),Ct!==null&&(t&&Ct.alternate!==null&&qe.delete(Ct.key===null?dt:Ct.key),X=f(Ct,X,dt),wt===null?Qe=Ct:wt.sibling=Ct,wt=Ct);return t&&qe.forEach(function(By){return n(Z,By)}),bt&&$i(Z,dt),Qe}function Gt(Z,X,J,ge){if(typeof J=="object"&&J!==null&&J.type===w&&J.key===null&&(J=J.props.children),typeof J=="object"&&J!==null){switch(J.$$typeof){case x:e:{for(var Qe=J.key;X!==null;){if(X.key===Qe){if(Qe=J.type,Qe===w){if(X.tag===7){a(Z,X.sibling),ge=u(X,J.props.children),ge.return=Z,Z=ge;break e}}else if(X.elementType===Qe||typeof Qe=="object"&&Qe!==null&&Qe.$$typeof===T&&vs(Qe)===X.type){a(Z,X.sibling),ge=u(X,J.props),lo(ge,J),ge.return=Z,Z=ge;break e}a(Z,X);break}else n(Z,X);X=X.sibling}J.type===w?(ge=hs(J.props.children,Z.mode,ge,J.key),ge.return=Z,Z=ge):(ge=ml(J.type,J.key,J.props,null,Z.mode,ge),lo(ge,J),ge.return=Z,Z=ge)}return _(Z);case E:e:{for(Qe=J.key;X!==null;){if(X.key===Qe)if(X.tag===4&&X.stateNode.containerInfo===J.containerInfo&&X.stateNode.implementation===J.implementation){a(Z,X.sibling),ge=u(X,J.children||[]),ge.return=Z,Z=ge;break e}else{a(Z,X);break}else n(Z,X);X=X.sibling}ge=Eu(J,Z.mode,ge),ge.return=Z,Z=ge}return _(Z);case T:return J=vs(J),Gt(Z,X,J,ge)}if(ee(J))return Ge(Z,X,J,ge);if(te(J)){if(Qe=te(J),typeof Qe!="function")throw Error(s(150));return J=Qe.call(J),Je(Z,X,J,ge)}if(typeof J.then=="function")return Gt(Z,X,bl(J),ge);if(J.$$typeof===U)return Gt(Z,X,vl(Z,J),ge);Ml(Z,J)}return typeof J=="string"&&J!==""||typeof J=="number"||typeof J=="bigint"?(J=""+J,X!==null&&X.tag===6?(a(Z,X.sibling),ge=u(X,J),ge.return=Z,Z=ge):(a(Z,X),ge=Mu(J,Z.mode,ge),ge.return=Z,Z=ge),_(Z)):a(Z,X)}return function(Z,X,J,ge){try{oo=0;var Qe=Gt(Z,X,J,ge);return ir=null,Qe}catch(qe){if(qe===nr||qe===yl)throw qe;var wt=Qn(29,qe,null,Z.mode);return wt.lanes=ge,wt.return=Z,wt}finally{}}}var ys=Kp(!0),Qp=Kp(!1),Ua=!1;function Iu(t){t.updateQueue={baseState:t.memoizedState,firstBaseUpdate:null,lastBaseUpdate:null,shared:{pending:null,lanes:0,hiddenCallbacks:null},callbacks:null}}function Fu(t,n){t=t.updateQueue,n.updateQueue===t&&(n.updateQueue={baseState:t.baseState,firstBaseUpdate:t.firstBaseUpdate,lastBaseUpdate:t.lastBaseUpdate,shared:t.shared,callbacks:null})}function La(t){return{lane:t,tag:0,payload:null,callback:null,next:null}}function Oa(t,n,a){var r=t.updateQueue;if(r===null)return null;if(r=r.shared,(Ut&2)!==0){var u=r.pending;return u===null?n.next=n:(n.next=u.next,u.next=n),r.pending=n,n=pl(t),Op(t,null,a),n}return hl(t,r,n,a),pl(t)}function co(t,n,a){if(n=n.updateQueue,n!==null&&(n=n.shared,(a&4194048)!==0)){var r=n.lanes;r&=t.pendingLanes,a|=r,n.lanes=a,zs(t,a)}}function zu(t,n){var a=t.updateQueue,r=t.alternate;if(r!==null&&(r=r.updateQueue,a===r)){var u=null,f=null;if(a=a.firstBaseUpdate,a!==null){do{var _={lane:a.lane,tag:a.tag,payload:a.payload,callback:null,next:null};f===null?u=f=_:f=f.next=_,a=a.next}while(a!==null);f===null?u=f=n:f=f.next=n}else u=f=n;a={baseState:r.baseState,firstBaseUpdate:u,lastBaseUpdate:f,shared:r.shared,callbacks:r.callbacks},t.updateQueue=a;return}t=a.lastBaseUpdate,t===null?a.firstBaseUpdate=n:t.next=n,a.lastBaseUpdate=n}var Bu=!1;function uo(){if(Bu){var t=tr;if(t!==null)throw t}}function fo(t,n,a,r){Bu=!1;var u=t.updateQueue;Ua=!1;var f=u.firstBaseUpdate,_=u.lastBaseUpdate,A=u.shared.pending;if(A!==null){u.shared.pending=null;var B=A,$=B.next;B.next=null,_===null?f=$:_.next=$,_=B;var he=t.alternate;he!==null&&(he=he.updateQueue,A=he.lastBaseUpdate,A!==_&&(A===null?he.firstBaseUpdate=$:A.next=$,he.lastBaseUpdate=B))}if(f!==null){var _e=u.baseState;_=0,he=$=B=null,A=f;do{var ae=A.lane&-536870913,oe=ae!==A.lane;if(oe?(yt&ae)===ae:(r&ae)===ae){ae!==0&&ae===er&&(Bu=!0),he!==null&&(he=he.next={lane:0,tag:A.tag,payload:A.payload,callback:null,next:null});e:{var Ge=t,Je=A;ae=n;var Gt=a;switch(Je.tag){case 1:if(Ge=Je.payload,typeof Ge=="function"){_e=Ge.call(Gt,_e,ae);break e}_e=Ge;break e;case 3:Ge.flags=Ge.flags&-65537|128;case 0:if(Ge=Je.payload,ae=typeof Ge=="function"?Ge.call(Gt,_e,ae):Ge,ae==null)break e;_e=y({},_e,ae);break e;case 2:Ua=!0}}ae=A.callback,ae!==null&&(t.flags|=64,oe&&(t.flags|=8192),oe=u.callbacks,oe===null?u.callbacks=[ae]:oe.push(ae))}else oe={lane:ae,tag:A.tag,payload:A.payload,callback:A.callback,next:null},he===null?($=he=oe,B=_e):he=he.next=oe,_|=ae;if(A=A.next,A===null){if(A=u.shared.pending,A===null)break;oe=A,A=oe.next,oe.next=null,u.lastBaseUpdate=oe,u.shared.pending=null}}while(!0);he===null&&(B=_e),u.baseState=B,u.firstBaseUpdate=$,u.lastBaseUpdate=he,f===null&&(u.shared.lanes=0),Ba|=_,t.lanes=_,t.memoizedState=_e}}function Jp(t,n){if(typeof t!="function")throw Error(s(191,t));t.call(n)}function $p(t,n){var a=t.callbacks;if(a!==null)for(t.callbacks=null,t=0;t<a.length;t++)Jp(a[t],n)}var ar=I(null),El=I(0);function em(t,n){t=ua,ve(El,t),ve(ar,n),ua=t|n.baseLanes}function Hu(){ve(El,ua),ve(ar,ar.current)}function Gu(){ua=El.current,Y(ar),Y(El)}var Jn=I(null),di=null;function Pa(t){var n=t.alternate;ve(an,an.current&1),ve(Jn,t),di===null&&(n===null||ar.current!==null||n.memoizedState!==null)&&(di=t)}function Vu(t){ve(an,an.current),ve(Jn,t),di===null&&(di=t)}function tm(t){t.tag===22?(ve(an,an.current),ve(Jn,t),di===null&&(di=t)):Ia()}function Ia(){ve(an,an.current),ve(Jn,Jn.current)}function $n(t){Y(Jn),di===t&&(di=null),Y(an)}var an=I(0);function Tl(t){for(var n=t;n!==null;){if(n.tag===13){var a=n.memoizedState;if(a!==null&&(a=a.dehydrated,a===null||Zf(a)||Kf(a)))return n}else if(n.tag===19&&(n.memoizedProps.revealOrder==="forwards"||n.memoizedProps.revealOrder==="backwards"||n.memoizedProps.revealOrder==="unstable_legacy-backwards"||n.memoizedProps.revealOrder==="together")){if((n.flags&128)!==0)return n}else if(n.child!==null){n.child.return=n,n=n.child;continue}if(n===t)break;for(;n.sibling===null;){if(n.return===null||n.return===t)return null;n=n.return}n.sibling.return=n.return,n=n.sibling}return null}var na=0,lt=null,Bt=null,un=null,Al=!1,sr=!1,Ss=!1,Rl=0,ho=0,rr=null,Cx=0;function en(){throw Error(s(321))}function ku(t,n){if(n===null)return!1;for(var a=0;a<n.length&&a<t.length;a++)if(!Kn(t[a],n[a]))return!1;return!0}function Xu(t,n,a,r,u,f){return na=f,lt=n,n.memoizedState=null,n.updateQueue=null,n.lanes=0,P.H=t===null||t.memoizedState===null?zm:rf,Ss=!1,f=a(r,u),Ss=!1,sr&&(f=im(n,a,r,u)),nm(t),f}function nm(t){P.H=go;var n=Bt!==null&&Bt.next!==null;if(na=0,un=Bt=lt=null,Al=!1,ho=0,rr=null,n)throw Error(s(300));t===null||fn||(t=t.dependencies,t!==null&&_l(t)&&(fn=!0))}function im(t,n,a,r){lt=t;var u=0;do{if(sr&&(rr=null),ho=0,sr=!1,25<=u)throw Error(s(301));if(u+=1,un=Bt=null,t.updateQueue!=null){var f=t.updateQueue;f.lastEffect=null,f.events=null,f.stores=null,f.memoCache!=null&&(f.memoCache.index=0)}P.H=Bm,f=n(a,r)}while(sr);return f}function Dx(){var t=P.H,n=t.useState()[0];return n=typeof n.then=="function"?po(n):n,t=t.useState()[0],(Bt!==null?Bt.memoizedState:null)!==t&&(lt.flags|=1024),n}function ju(){var t=Rl!==0;return Rl=0,t}function Wu(t,n,a){n.updateQueue=t.updateQueue,n.flags&=-2053,t.lanes&=~a}function qu(t){if(Al){for(t=t.memoizedState;t!==null;){var n=t.queue;n!==null&&(n.pending=null),t=t.next}Al=!1}na=0,un=Bt=lt=null,sr=!1,ho=Rl=0,rr=null}function Pn(){var t={memoizedState:null,baseState:null,baseQueue:null,queue:null,next:null};return un===null?lt.memoizedState=un=t:un=un.next=t,un}function sn(){if(Bt===null){var t=lt.alternate;t=t!==null?t.memoizedState:null}else t=Bt.next;var n=un===null?lt.memoizedState:un.next;if(n!==null)un=n,Bt=t;else{if(t===null)throw lt.alternate===null?Error(s(467)):Error(s(310));Bt=t,t={memoizedState:Bt.memoizedState,baseState:Bt.baseState,baseQueue:Bt.baseQueue,queue:Bt.queue,next:null},un===null?lt.memoizedState=un=t:un=un.next=t}return un}function wl(){return{lastEffect:null,events:null,stores:null,memoCache:null}}function po(t){var n=ho;return ho+=1,rr===null&&(rr=[]),t=qp(rr,t,n),n=lt,(un===null?n.memoizedState:un.next)===null&&(n=n.alternate,P.H=n===null||n.memoizedState===null?zm:rf),t}function Cl(t){if(t!==null&&typeof t=="object"){if(typeof t.then=="function")return po(t);if(t.$$typeof===U)return Mn(t)}throw Error(s(438,String(t)))}function Yu(t){var n=null,a=lt.updateQueue;if(a!==null&&(n=a.memoCache),n==null){var r=lt.alternate;r!==null&&(r=r.updateQueue,r!==null&&(r=r.memoCache,r!=null&&(n={data:r.data.map(function(u){return u.slice()}),index:0})))}if(n==null&&(n={data:[],index:0}),a===null&&(a=wl(),lt.updateQueue=a),a.memoCache=n,a=n.data[n.index],a===void 0)for(a=n.data[n.index]=Array(t),r=0;r<t;r++)a[r]=le;return n.index++,a}function ia(t,n){return typeof n=="function"?n(t):n}function Dl(t){var n=sn();return Zu(n,Bt,t)}function Zu(t,n,a){var r=t.queue;if(r===null)throw Error(s(311));r.lastRenderedReducer=a;var u=t.baseQueue,f=r.pending;if(f!==null){if(u!==null){var _=u.next;u.next=f.next,f.next=_}n.baseQueue=u=f,r.pending=null}if(f=t.baseState,u===null)t.memoizedState=f;else{n=u.next;var A=_=null,B=null,$=n,he=!1;do{var _e=$.lane&-536870913;if(_e!==$.lane?(yt&_e)===_e:(na&_e)===_e){var ae=$.revertLane;if(ae===0)B!==null&&(B=B.next={lane:0,revertLane:0,gesture:null,action:$.action,hasEagerState:$.hasEagerState,eagerState:$.eagerState,next:null}),_e===er&&(he=!0);else if((na&ae)===ae){$=$.next,ae===er&&(he=!0);continue}else _e={lane:0,revertLane:$.revertLane,gesture:null,action:$.action,hasEagerState:$.hasEagerState,eagerState:$.eagerState,next:null},B===null?(A=B=_e,_=f):B=B.next=_e,lt.lanes|=ae,Ba|=ae;_e=$.action,Ss&&a(f,_e),f=$.hasEagerState?$.eagerState:a(f,_e)}else ae={lane:_e,revertLane:$.revertLane,gesture:$.gesture,action:$.action,hasEagerState:$.hasEagerState,eagerState:$.eagerState,next:null},B===null?(A=B=ae,_=f):B=B.next=ae,lt.lanes|=_e,Ba|=_e;$=$.next}while($!==null&&$!==n);if(B===null?_=f:B.next=A,!Kn(f,t.memoizedState)&&(fn=!0,he&&(a=tr,a!==null)))throw a;t.memoizedState=f,t.baseState=_,t.baseQueue=B,r.lastRenderedState=f}return u===null&&(r.lanes=0),[t.memoizedState,r.dispatch]}function Ku(t){var n=sn(),a=n.queue;if(a===null)throw Error(s(311));a.lastRenderedReducer=t;var r=a.dispatch,u=a.pending,f=n.memoizedState;if(u!==null){a.pending=null;var _=u=u.next;do f=t(f,_.action),_=_.next;while(_!==u);Kn(f,n.memoizedState)||(fn=!0),n.memoizedState=f,n.baseQueue===null&&(n.baseState=f),a.lastRenderedState=f}return[f,r]}function am(t,n,a){var r=lt,u=sn(),f=bt;if(f){if(a===void 0)throw Error(s(407));a=a()}else a=n();var _=!Kn((Bt||u).memoizedState,a);if(_&&(u.memoizedState=a,fn=!0),u=u.queue,$u(om.bind(null,r,u,t),[t]),u.getSnapshot!==n||_||un!==null&&un.memoizedState.tag&1){if(r.flags|=2048,or(9,{destroy:void 0},rm.bind(null,r,u,a,n),null),kt===null)throw Error(s(349));f||(na&127)!==0||sm(r,n,a)}return a}function sm(t,n,a){t.flags|=16384,t={getSnapshot:n,value:a},n=lt.updateQueue,n===null?(n=wl(),lt.updateQueue=n,n.stores=[t]):(a=n.stores,a===null?n.stores=[t]:a.push(t))}function rm(t,n,a,r){n.value=a,n.getSnapshot=r,lm(n)&&cm(t)}function om(t,n,a){return a(function(){lm(n)&&cm(t)})}function lm(t){var n=t.getSnapshot;t=t.value;try{var a=n();return!Kn(t,a)}catch{return!0}}function cm(t){var n=ds(t,2);n!==null&&Xn(n,t,2)}function Qu(t){var n=Pn();if(typeof t=="function"){var a=t;if(t=a(),Ss){Oe(!0);try{a()}finally{Oe(!1)}}}return n.memoizedState=n.baseState=t,n.queue={pending:null,lanes:0,dispatch:null,lastRenderedReducer:ia,lastRenderedState:t},n}function um(t,n,a,r){return t.baseState=a,Zu(t,Bt,typeof r=="function"?r:ia)}function Nx(t,n,a,r,u){if(Ll(t))throw Error(s(485));if(t=n.action,t!==null){var f={payload:u,action:t,next:null,isTransition:!0,status:"pending",value:null,reason:null,listeners:[],then:function(_){f.listeners.push(_)}};P.T!==null?a(!0):f.isTransition=!1,r(f),a=n.pending,a===null?(f.next=n.pending=f,fm(n,f)):(f.next=a.next,n.pending=a.next=f)}}function fm(t,n){var a=n.action,r=n.payload,u=t.state;if(n.isTransition){var f=P.T,_={};P.T=_;try{var A=a(u,r),B=P.S;B!==null&&B(_,A),dm(t,n,A)}catch($){Ju(t,n,$)}finally{f!==null&&_.types!==null&&(f.types=_.types),P.T=f}}else try{f=a(u,r),dm(t,n,f)}catch($){Ju(t,n,$)}}function dm(t,n,a){a!==null&&typeof a=="object"&&typeof a.then=="function"?a.then(function(r){hm(t,n,r)},function(r){return Ju(t,n,r)}):hm(t,n,a)}function hm(t,n,a){n.status="fulfilled",n.value=a,pm(n),t.state=a,n=t.pending,n!==null&&(a=n.next,a===n?t.pending=null:(a=a.next,n.next=a,fm(t,a)))}function Ju(t,n,a){var r=t.pending;if(t.pending=null,r!==null){r=r.next;do n.status="rejected",n.reason=a,pm(n),n=n.next;while(n!==r)}t.action=null}function pm(t){t=t.listeners;for(var n=0;n<t.length;n++)(0,t[n])()}function mm(t,n){return n}function gm(t,n){if(bt){var a=kt.formState;if(a!==null){e:{var r=lt;if(bt){if(Xt){t:{for(var u=Xt,f=fi;u.nodeType!==8;){if(!f){u=null;break t}if(u=hi(u.nextSibling),u===null){u=null;break t}}f=u.data,u=f==="F!"||f==="F"?u:null}if(u){Xt=hi(u.nextSibling),r=u.data==="F!";break e}}Da(r)}r=!1}r&&(n=a[0])}}return a=Pn(),a.memoizedState=a.baseState=n,r={pending:null,lanes:0,dispatch:null,lastRenderedReducer:mm,lastRenderedState:n},a.queue=r,a=Pm.bind(null,lt,r),r.dispatch=a,r=Qu(!1),f=sf.bind(null,lt,!1,r.queue),r=Pn(),u={state:n,dispatch:null,action:t,pending:null},r.queue=u,a=Nx.bind(null,lt,u,f,a),u.dispatch=a,r.memoizedState=t,[n,a,!1]}function _m(t){var n=sn();return vm(n,Bt,t)}function vm(t,n,a){if(n=Zu(t,n,mm)[0],t=Dl(ia)[0],typeof n=="object"&&n!==null&&typeof n.then=="function")try{var r=po(n)}catch(_){throw _===nr?yl:_}else r=n;n=sn();var u=n.queue,f=u.dispatch;return a!==n.memoizedState&&(lt.flags|=2048,or(9,{destroy:void 0},Ux.bind(null,u,a),null)),[r,f,t]}function Ux(t,n){t.action=n}function xm(t){var n=sn(),a=Bt;if(a!==null)return vm(n,a,t);sn(),n=n.memoizedState,a=sn();var r=a.queue.dispatch;return a.memoizedState=t,[n,r,!1]}function or(t,n,a,r){return t={tag:t,create:a,deps:r,inst:n,next:null},n=lt.updateQueue,n===null&&(n=wl(),lt.updateQueue=n),a=n.lastEffect,a===null?n.lastEffect=t.next=t:(r=a.next,a.next=t,t.next=r,n.lastEffect=t),t}function ym(){return sn().memoizedState}function Nl(t,n,a,r){var u=Pn();lt.flags|=t,u.memoizedState=or(1|n,{destroy:void 0},a,r===void 0?null:r)}function Ul(t,n,a,r){var u=sn();r=r===void 0?null:r;var f=u.memoizedState.inst;Bt!==null&&r!==null&&ku(r,Bt.memoizedState.deps)?u.memoizedState=or(n,f,a,r):(lt.flags|=t,u.memoizedState=or(1|n,f,a,r))}function Sm(t,n){Nl(8390656,8,t,n)}function $u(t,n){Ul(2048,8,t,n)}function Lx(t){lt.flags|=4;var n=lt.updateQueue;if(n===null)n=wl(),lt.updateQueue=n,n.events=[t];else{var a=n.events;a===null?n.events=[t]:a.push(t)}}function bm(t){var n=sn().memoizedState;return Lx({ref:n,nextImpl:t}),function(){if((Ut&2)!==0)throw Error(s(440));return n.impl.apply(void 0,arguments)}}function Mm(t,n){return Ul(4,2,t,n)}function Em(t,n){return Ul(4,4,t,n)}function Tm(t,n){if(typeof n=="function"){t=t();var a=n(t);return function(){typeof a=="function"?a():n(null)}}if(n!=null)return t=t(),n.current=t,function(){n.current=null}}function Am(t,n,a){a=a!=null?a.concat([t]):null,Ul(4,4,Tm.bind(null,n,t),a)}function ef(){}function Rm(t,n){var a=sn();n=n===void 0?null:n;var r=a.memoizedState;return n!==null&&ku(n,r[1])?r[0]:(a.memoizedState=[t,n],t)}function wm(t,n){var a=sn();n=n===void 0?null:n;var r=a.memoizedState;if(n!==null&&ku(n,r[1]))return r[0];if(r=t(),Ss){Oe(!0);try{t()}finally{Oe(!1)}}return a.memoizedState=[r,n],r}function tf(t,n,a){return a===void 0||(na&1073741824)!==0&&(yt&261930)===0?t.memoizedState=n:(t.memoizedState=a,t=Cg(),lt.lanes|=t,Ba|=t,a)}function Cm(t,n,a,r){return Kn(a,n)?a:ar.current!==null?(t=tf(t,a,r),Kn(t,n)||(fn=!0),t):(na&42)===0||(na&1073741824)!==0&&(yt&261930)===0?(fn=!0,t.memoizedState=a):(t=Cg(),lt.lanes|=t,Ba|=t,n)}function Dm(t,n,a,r,u){var f=z.p;z.p=f!==0&&8>f?f:8;var _=P.T,A={};P.T=A,sf(t,!1,n,a);try{var B=u(),$=P.S;if($!==null&&$(A,B),B!==null&&typeof B=="object"&&typeof B.then=="function"){var he=wx(B,r);mo(t,n,he,ni(t))}else mo(t,n,r,ni(t))}catch(_e){mo(t,n,{then:function(){},status:"rejected",reason:_e},ni())}finally{z.p=f,_!==null&&A.types!==null&&(_.types=A.types),P.T=_}}function Ox(){}function nf(t,n,a,r){if(t.tag!==5)throw Error(s(476));var u=Nm(t).queue;Dm(t,u,n,ce,a===null?Ox:function(){return Um(t),a(r)})}function Nm(t){var n=t.memoizedState;if(n!==null)return n;n={memoizedState:ce,baseState:ce,baseQueue:null,queue:{pending:null,lanes:0,dispatch:null,lastRenderedReducer:ia,lastRenderedState:ce},next:null};var a={};return n.next={memoizedState:a,baseState:a,baseQueue:null,queue:{pending:null,lanes:0,dispatch:null,lastRenderedReducer:ia,lastRenderedState:a},next:null},t.memoizedState=n,t=t.alternate,t!==null&&(t.memoizedState=n),n}function Um(t){var n=Nm(t);n.next===null&&(n=t.alternate.memoizedState),mo(t,n.next.queue,{},ni())}function af(){return Mn(Uo)}function Lm(){return sn().memoizedState}function Om(){return sn().memoizedState}function Px(t){for(var n=t.return;n!==null;){switch(n.tag){case 24:case 3:var a=ni();t=La(a);var r=Oa(n,t,a);r!==null&&(Xn(r,n,a),co(r,n,a)),n={cache:Uu()},t.payload=n;return}n=n.return}}function Ix(t,n,a){var r=ni();a={lane:r,revertLane:0,gesture:null,action:a,hasEagerState:!1,eagerState:null,next:null},Ll(t)?Im(n,a):(a=Su(t,n,a,r),a!==null&&(Xn(a,t,r),Fm(a,n,r)))}function Pm(t,n,a){var r=ni();mo(t,n,a,r)}function mo(t,n,a,r){var u={lane:r,revertLane:0,gesture:null,action:a,hasEagerState:!1,eagerState:null,next:null};if(Ll(t))Im(n,u);else{var f=t.alternate;if(t.lanes===0&&(f===null||f.lanes===0)&&(f=n.lastRenderedReducer,f!==null))try{var _=n.lastRenderedState,A=f(_,a);if(u.hasEagerState=!0,u.eagerState=A,Kn(A,_))return hl(t,n,u,0),kt===null&&dl(),!1}catch{}finally{}if(a=Su(t,n,u,r),a!==null)return Xn(a,t,r),Fm(a,n,r),!0}return!1}function sf(t,n,a,r){if(r={lane:2,revertLane:Ff(),gesture:null,action:r,hasEagerState:!1,eagerState:null,next:null},Ll(t)){if(n)throw Error(s(479))}else n=Su(t,a,r,2),n!==null&&Xn(n,t,2)}function Ll(t){var n=t.alternate;return t===lt||n!==null&&n===lt}function Im(t,n){sr=Al=!0;var a=t.pending;a===null?n.next=n:(n.next=a.next,a.next=n),t.pending=n}function Fm(t,n,a){if((a&4194048)!==0){var r=n.lanes;r&=t.pendingLanes,a|=r,n.lanes=a,zs(t,a)}}var go={readContext:Mn,use:Cl,useCallback:en,useContext:en,useEffect:en,useImperativeHandle:en,useLayoutEffect:en,useInsertionEffect:en,useMemo:en,useReducer:en,useRef:en,useState:en,useDebugValue:en,useDeferredValue:en,useTransition:en,useSyncExternalStore:en,useId:en,useHostTransitionStatus:en,useFormState:en,useActionState:en,useOptimistic:en,useMemoCache:en,useCacheRefresh:en};go.useEffectEvent=en;var zm={readContext:Mn,use:Cl,useCallback:function(t,n){return Pn().memoizedState=[t,n===void 0?null:n],t},useContext:Mn,useEffect:Sm,useImperativeHandle:function(t,n,a){a=a!=null?a.concat([t]):null,Nl(4194308,4,Tm.bind(null,n,t),a)},useLayoutEffect:function(t,n){return Nl(4194308,4,t,n)},useInsertionEffect:function(t,n){Nl(4,2,t,n)},useMemo:function(t,n){var a=Pn();n=n===void 0?null:n;var r=t();if(Ss){Oe(!0);try{t()}finally{Oe(!1)}}return a.memoizedState=[r,n],r},useReducer:function(t,n,a){var r=Pn();if(a!==void 0){var u=a(n);if(Ss){Oe(!0);try{a(n)}finally{Oe(!1)}}}else u=n;return r.memoizedState=r.baseState=u,t={pending:null,lanes:0,dispatch:null,lastRenderedReducer:t,lastRenderedState:u},r.queue=t,t=t.dispatch=Ix.bind(null,lt,t),[r.memoizedState,t]},useRef:function(t){var n=Pn();return t={current:t},n.memoizedState=t},useState:function(t){t=Qu(t);var n=t.queue,a=Pm.bind(null,lt,n);return n.dispatch=a,[t.memoizedState,a]},useDebugValue:ef,useDeferredValue:function(t,n){var a=Pn();return tf(a,t,n)},useTransition:function(){var t=Qu(!1);return t=Dm.bind(null,lt,t.queue,!0,!1),Pn().memoizedState=t,[!1,t]},useSyncExternalStore:function(t,n,a){var r=lt,u=Pn();if(bt){if(a===void 0)throw Error(s(407));a=a()}else{if(a=n(),kt===null)throw Error(s(349));(yt&127)!==0||sm(r,n,a)}u.memoizedState=a;var f={value:a,getSnapshot:n};return u.queue=f,Sm(om.bind(null,r,f,t),[t]),r.flags|=2048,or(9,{destroy:void 0},rm.bind(null,r,f,a,n),null),a},useId:function(){var t=Pn(),n=kt.identifierPrefix;if(bt){var a=Oi,r=Li;a=(r&~(1<<32-Le(r)-1)).toString(32)+a,n="_"+n+"R_"+a,a=Rl++,0<a&&(n+="H"+a.toString(32)),n+="_"}else a=Cx++,n="_"+n+"r_"+a.toString(32)+"_";return t.memoizedState=n},useHostTransitionStatus:af,useFormState:gm,useActionState:gm,useOptimistic:function(t){var n=Pn();n.memoizedState=n.baseState=t;var a={pending:null,lanes:0,dispatch:null,lastRenderedReducer:null,lastRenderedState:null};return n.queue=a,n=sf.bind(null,lt,!0,a),a.dispatch=n,[t,n]},useMemoCache:Yu,useCacheRefresh:function(){return Pn().memoizedState=Px.bind(null,lt)},useEffectEvent:function(t){var n=Pn(),a={impl:t};return n.memoizedState=a,function(){if((Ut&2)!==0)throw Error(s(440));return a.impl.apply(void 0,arguments)}}},rf={readContext:Mn,use:Cl,useCallback:Rm,useContext:Mn,useEffect:$u,useImperativeHandle:Am,useInsertionEffect:Mm,useLayoutEffect:Em,useMemo:wm,useReducer:Dl,useRef:ym,useState:function(){return Dl(ia)},useDebugValue:ef,useDeferredValue:function(t,n){var a=sn();return Cm(a,Bt.memoizedState,t,n)},useTransition:function(){var t=Dl(ia)[0],n=sn().memoizedState;return[typeof t=="boolean"?t:po(t),n]},useSyncExternalStore:am,useId:Lm,useHostTransitionStatus:af,useFormState:_m,useActionState:_m,useOptimistic:function(t,n){var a=sn();return um(a,Bt,t,n)},useMemoCache:Yu,useCacheRefresh:Om};rf.useEffectEvent=bm;var Bm={readContext:Mn,use:Cl,useCallback:Rm,useContext:Mn,useEffect:$u,useImperativeHandle:Am,useInsertionEffect:Mm,useLayoutEffect:Em,useMemo:wm,useReducer:Ku,useRef:ym,useState:function(){return Ku(ia)},useDebugValue:ef,useDeferredValue:function(t,n){var a=sn();return Bt===null?tf(a,t,n):Cm(a,Bt.memoizedState,t,n)},useTransition:function(){var t=Ku(ia)[0],n=sn().memoizedState;return[typeof t=="boolean"?t:po(t),n]},useSyncExternalStore:am,useId:Lm,useHostTransitionStatus:af,useFormState:xm,useActionState:xm,useOptimistic:function(t,n){var a=sn();return Bt!==null?um(a,Bt,t,n):(a.baseState=t,[t,a.queue.dispatch])},useMemoCache:Yu,useCacheRefresh:Om};Bm.useEffectEvent=bm;function of(t,n,a,r){n=t.memoizedState,a=a(r,n),a=a==null?n:y({},n,a),t.memoizedState=a,t.lanes===0&&(t.updateQueue.baseState=a)}var lf={enqueueSetState:function(t,n,a){t=t._reactInternals;var r=ni(),u=La(r);u.payload=n,a!=null&&(u.callback=a),n=Oa(t,u,r),n!==null&&(Xn(n,t,r),co(n,t,r))},enqueueReplaceState:function(t,n,a){t=t._reactInternals;var r=ni(),u=La(r);u.tag=1,u.payload=n,a!=null&&(u.callback=a),n=Oa(t,u,r),n!==null&&(Xn(n,t,r),co(n,t,r))},enqueueForceUpdate:function(t,n){t=t._reactInternals;var a=ni(),r=La(a);r.tag=2,n!=null&&(r.callback=n),n=Oa(t,r,a),n!==null&&(Xn(n,t,a),co(n,t,a))}};function Hm(t,n,a,r,u,f,_){return t=t.stateNode,typeof t.shouldComponentUpdate=="function"?t.shouldComponentUpdate(r,f,_):n.prototype&&n.prototype.isPureReactComponent?!to(a,r)||!to(u,f):!0}function Gm(t,n,a,r){t=n.state,typeof n.componentWillReceiveProps=="function"&&n.componentWillReceiveProps(a,r),typeof n.UNSAFE_componentWillReceiveProps=="function"&&n.UNSAFE_componentWillReceiveProps(a,r),n.state!==t&&lf.enqueueReplaceState(n,n.state,null)}function bs(t,n){var a=n;if("ref"in n){a={};for(var r in n)r!=="ref"&&(a[r]=n[r])}if(t=t.defaultProps){a===n&&(a=y({},a));for(var u in t)a[u]===void 0&&(a[u]=t[u])}return a}function Vm(t){fl(t)}function km(t){console.error(t)}function Xm(t){fl(t)}function Ol(t,n){try{var a=t.onUncaughtError;a(n.value,{componentStack:n.stack})}catch(r){setTimeout(function(){throw r})}}function jm(t,n,a){try{var r=t.onCaughtError;r(a.value,{componentStack:a.stack,errorBoundary:n.tag===1?n.stateNode:null})}catch(u){setTimeout(function(){throw u})}}function cf(t,n,a){return a=La(a),a.tag=3,a.payload={element:null},a.callback=function(){Ol(t,n)},a}function Wm(t){return t=La(t),t.tag=3,t}function qm(t,n,a,r){var u=a.type.getDerivedStateFromError;if(typeof u=="function"){var f=r.value;t.payload=function(){return u(f)},t.callback=function(){jm(n,a,r)}}var _=a.stateNode;_!==null&&typeof _.componentDidCatch=="function"&&(t.callback=function(){jm(n,a,r),typeof u!="function"&&(Ha===null?Ha=new Set([this]):Ha.add(this));var A=r.stack;this.componentDidCatch(r.value,{componentStack:A!==null?A:""})})}function Fx(t,n,a,r,u){if(a.flags|=32768,r!==null&&typeof r=="object"&&typeof r.then=="function"){if(n=a.alternate,n!==null&&$s(n,a,u,!0),a=Jn.current,a!==null){switch(a.tag){case 31:case 13:return di===null?Wl():a.alternate===null&&tn===0&&(tn=3),a.flags&=-257,a.flags|=65536,a.lanes=u,r===Sl?a.flags|=16384:(n=a.updateQueue,n===null?a.updateQueue=new Set([r]):n.add(r),Of(t,r,u)),!1;case 22:return a.flags|=65536,r===Sl?a.flags|=16384:(n=a.updateQueue,n===null?(n={transitions:null,markerInstances:null,retryQueue:new Set([r])},a.updateQueue=n):(a=n.retryQueue,a===null?n.retryQueue=new Set([r]):a.add(r)),Of(t,r,u)),!1}throw Error(s(435,a.tag))}return Of(t,r,u),Wl(),!1}if(bt)return n=Jn.current,n!==null?((n.flags&65536)===0&&(n.flags|=256),n.flags|=65536,n.lanes=u,r!==Ru&&(t=Error(s(422),{cause:r}),ao(li(t,a)))):(r!==Ru&&(n=Error(s(423),{cause:r}),ao(li(n,a))),t=t.current.alternate,t.flags|=65536,u&=-u,t.lanes|=u,r=li(r,a),u=cf(t.stateNode,r,u),zu(t,u),tn!==4&&(tn=2)),!1;var f=Error(s(520),{cause:r});if(f=li(f,a),Eo===null?Eo=[f]:Eo.push(f),tn!==4&&(tn=2),n===null)return!0;r=li(r,a),a=n;do{switch(a.tag){case 3:return a.flags|=65536,t=u&-u,a.lanes|=t,t=cf(a.stateNode,r,t),zu(a,t),!1;case 1:if(n=a.type,f=a.stateNode,(a.flags&128)===0&&(typeof n.getDerivedStateFromError=="function"||f!==null&&typeof f.componentDidCatch=="function"&&(Ha===null||!Ha.has(f))))return a.flags|=65536,u&=-u,a.lanes|=u,u=Wm(u),qm(u,t,a,r),zu(a,u),!1}a=a.return}while(a!==null);return!1}var uf=Error(s(461)),fn=!1;function En(t,n,a,r){n.child=t===null?Qp(n,null,a,r):ys(n,t.child,a,r)}function Ym(t,n,a,r,u){a=a.render;var f=n.ref;if("ref"in r){var _={};for(var A in r)A!=="ref"&&(_[A]=r[A])}else _=r;return gs(n),r=Xu(t,n,a,_,f,u),A=ju(),t!==null&&!fn?(Wu(t,n,u),aa(t,n,u)):(bt&&A&&Tu(n),n.flags|=1,En(t,n,r,u),n.child)}function Zm(t,n,a,r,u){if(t===null){var f=a.type;return typeof f=="function"&&!bu(f)&&f.defaultProps===void 0&&a.compare===null?(n.tag=15,n.type=f,Km(t,n,f,r,u)):(t=ml(a.type,null,r,n,n.mode,u),t.ref=n.ref,t.return=n,n.child=t)}if(f=t.child,!vf(t,u)){var _=f.memoizedProps;if(a=a.compare,a=a!==null?a:to,a(_,r)&&t.ref===n.ref)return aa(t,n,u)}return n.flags|=1,t=Ji(f,r),t.ref=n.ref,t.return=n,n.child=t}function Km(t,n,a,r,u){if(t!==null){var f=t.memoizedProps;if(to(f,r)&&t.ref===n.ref)if(fn=!1,n.pendingProps=r=f,vf(t,u))(t.flags&131072)!==0&&(fn=!0);else return n.lanes=t.lanes,aa(t,n,u)}return ff(t,n,a,r,u)}function Qm(t,n,a,r){var u=r.children,f=t!==null?t.memoizedState:null;if(t===null&&n.stateNode===null&&(n.stateNode={_visibility:1,_pendingMarkers:null,_retryCache:null,_transitions:null}),r.mode==="hidden"){if((n.flags&128)!==0){if(f=f!==null?f.baseLanes|a:a,t!==null){for(r=n.child=t.child,u=0;r!==null;)u=u|r.lanes|r.childLanes,r=r.sibling;r=u&~f}else r=0,n.child=null;return Jm(t,n,f,a,r)}if((a&536870912)!==0)n.memoizedState={baseLanes:0,cachePool:null},t!==null&&xl(n,f!==null?f.cachePool:null),f!==null?em(n,f):Hu(),tm(n);else return r=n.lanes=536870912,Jm(t,n,f!==null?f.baseLanes|a:a,a,r)}else f!==null?(xl(n,f.cachePool),em(n,f),Ia(),n.memoizedState=null):(t!==null&&xl(n,null),Hu(),Ia());return En(t,n,u,a),n.child}function _o(t,n){return t!==null&&t.tag===22||n.stateNode!==null||(n.stateNode={_visibility:1,_pendingMarkers:null,_retryCache:null,_transitions:null}),n.sibling}function Jm(t,n,a,r,u){var f=Ou();return f=f===null?null:{parent:cn._currentValue,pool:f},n.memoizedState={baseLanes:a,cachePool:f},t!==null&&xl(n,null),Hu(),tm(n),t!==null&&$s(t,n,r,!0),n.childLanes=u,null}function Pl(t,n){return n=Fl({mode:n.mode,children:n.children},t.mode),n.ref=t.ref,t.child=n,n.return=t,n}function $m(t,n,a){return ys(n,t.child,null,a),t=Pl(n,n.pendingProps),t.flags|=2,$n(n),n.memoizedState=null,t}function zx(t,n,a){var r=n.pendingProps,u=(n.flags&128)!==0;if(n.flags&=-129,t===null){if(bt){if(r.mode==="hidden")return t=Pl(n,r),n.lanes=536870912,_o(null,t);if(Vu(n),(t=Xt)?(t=f0(t,fi),t=t!==null&&t.data==="&"?t:null,t!==null&&(n.memoizedState={dehydrated:t,treeContext:wa!==null?{id:Li,overflow:Oi}:null,retryLane:536870912,hydrationErrors:null},a=Ip(t),a.return=n,n.child=a,bn=n,Xt=null)):t=null,t===null)throw Da(n);return n.lanes=536870912,null}return Pl(n,r)}var f=t.memoizedState;if(f!==null){var _=f.dehydrated;if(Vu(n),u)if(n.flags&256)n.flags&=-257,n=$m(t,n,a);else if(n.memoizedState!==null)n.child=t.child,n.flags|=128,n=null;else throw Error(s(558));else if(fn||$s(t,n,a,!1),u=(a&t.childLanes)!==0,fn||u){if(r=kt,r!==null&&(_=el(r,a),_!==0&&_!==f.retryLane))throw f.retryLane=_,ds(t,_),Xn(r,t,_),uf;Wl(),n=$m(t,n,a)}else t=f.treeContext,Xt=hi(_.nextSibling),bn=n,bt=!0,Ca=null,fi=!1,t!==null&&Bp(n,t),n=Pl(n,r),n.flags|=4096;return n}return t=Ji(t.child,{mode:r.mode,children:r.children}),t.ref=n.ref,n.child=t,t.return=n,t}function Il(t,n){var a=n.ref;if(a===null)t!==null&&t.ref!==null&&(n.flags|=4194816);else{if(typeof a!="function"&&typeof a!="object")throw Error(s(284));(t===null||t.ref!==a)&&(n.flags|=4194816)}}function ff(t,n,a,r,u){return gs(n),a=Xu(t,n,a,r,void 0,u),r=ju(),t!==null&&!fn?(Wu(t,n,u),aa(t,n,u)):(bt&&r&&Tu(n),n.flags|=1,En(t,n,a,u),n.child)}function eg(t,n,a,r,u,f){return gs(n),n.updateQueue=null,a=im(n,r,a,u),nm(t),r=ju(),t!==null&&!fn?(Wu(t,n,f),aa(t,n,f)):(bt&&r&&Tu(n),n.flags|=1,En(t,n,a,f),n.child)}function tg(t,n,a,r,u){if(gs(n),n.stateNode===null){var f=Zs,_=a.contextType;typeof _=="object"&&_!==null&&(f=Mn(_)),f=new a(r,f),n.memoizedState=f.state!==null&&f.state!==void 0?f.state:null,f.updater=lf,n.stateNode=f,f._reactInternals=n,f=n.stateNode,f.props=r,f.state=n.memoizedState,f.refs={},Iu(n),_=a.contextType,f.context=typeof _=="object"&&_!==null?Mn(_):Zs,f.state=n.memoizedState,_=a.getDerivedStateFromProps,typeof _=="function"&&(of(n,a,_,r),f.state=n.memoizedState),typeof a.getDerivedStateFromProps=="function"||typeof f.getSnapshotBeforeUpdate=="function"||typeof f.UNSAFE_componentWillMount!="function"&&typeof f.componentWillMount!="function"||(_=f.state,typeof f.componentWillMount=="function"&&f.componentWillMount(),typeof f.UNSAFE_componentWillMount=="function"&&f.UNSAFE_componentWillMount(),_!==f.state&&lf.enqueueReplaceState(f,f.state,null),fo(n,r,f,u),uo(),f.state=n.memoizedState),typeof f.componentDidMount=="function"&&(n.flags|=4194308),r=!0}else if(t===null){f=n.stateNode;var A=n.memoizedProps,B=bs(a,A);f.props=B;var $=f.context,he=a.contextType;_=Zs,typeof he=="object"&&he!==null&&(_=Mn(he));var _e=a.getDerivedStateFromProps;he=typeof _e=="function"||typeof f.getSnapshotBeforeUpdate=="function",A=n.pendingProps!==A,he||typeof f.UNSAFE_componentWillReceiveProps!="function"&&typeof f.componentWillReceiveProps!="function"||(A||$!==_)&&Gm(n,f,r,_),Ua=!1;var ae=n.memoizedState;f.state=ae,fo(n,r,f,u),uo(),$=n.memoizedState,A||ae!==$||Ua?(typeof _e=="function"&&(of(n,a,_e,r),$=n.memoizedState),(B=Ua||Hm(n,a,B,r,ae,$,_))?(he||typeof f.UNSAFE_componentWillMount!="function"&&typeof f.componentWillMount!="function"||(typeof f.componentWillMount=="function"&&f.componentWillMount(),typeof f.UNSAFE_componentWillMount=="function"&&f.UNSAFE_componentWillMount()),typeof f.componentDidMount=="function"&&(n.flags|=4194308)):(typeof f.componentDidMount=="function"&&(n.flags|=4194308),n.memoizedProps=r,n.memoizedState=$),f.props=r,f.state=$,f.context=_,r=B):(typeof f.componentDidMount=="function"&&(n.flags|=4194308),r=!1)}else{f=n.stateNode,Fu(t,n),_=n.memoizedProps,he=bs(a,_),f.props=he,_e=n.pendingProps,ae=f.context,$=a.contextType,B=Zs,typeof $=="object"&&$!==null&&(B=Mn($)),A=a.getDerivedStateFromProps,($=typeof A=="function"||typeof f.getSnapshotBeforeUpdate=="function")||typeof f.UNSAFE_componentWillReceiveProps!="function"&&typeof f.componentWillReceiveProps!="function"||(_!==_e||ae!==B)&&Gm(n,f,r,B),Ua=!1,ae=n.memoizedState,f.state=ae,fo(n,r,f,u),uo();var oe=n.memoizedState;_!==_e||ae!==oe||Ua||t!==null&&t.dependencies!==null&&_l(t.dependencies)?(typeof A=="function"&&(of(n,a,A,r),oe=n.memoizedState),(he=Ua||Hm(n,a,he,r,ae,oe,B)||t!==null&&t.dependencies!==null&&_l(t.dependencies))?($||typeof f.UNSAFE_componentWillUpdate!="function"&&typeof f.componentWillUpdate!="function"||(typeof f.componentWillUpdate=="function"&&f.componentWillUpdate(r,oe,B),typeof f.UNSAFE_componentWillUpdate=="function"&&f.UNSAFE_componentWillUpdate(r,oe,B)),typeof f.componentDidUpdate=="function"&&(n.flags|=4),typeof f.getSnapshotBeforeUpdate=="function"&&(n.flags|=1024)):(typeof f.componentDidUpdate!="function"||_===t.memoizedProps&&ae===t.memoizedState||(n.flags|=4),typeof f.getSnapshotBeforeUpdate!="function"||_===t.memoizedProps&&ae===t.memoizedState||(n.flags|=1024),n.memoizedProps=r,n.memoizedState=oe),f.props=r,f.state=oe,f.context=B,r=he):(typeof f.componentDidUpdate!="function"||_===t.memoizedProps&&ae===t.memoizedState||(n.flags|=4),typeof f.getSnapshotBeforeUpdate!="function"||_===t.memoizedProps&&ae===t.memoizedState||(n.flags|=1024),r=!1)}return f=r,Il(t,n),r=(n.flags&128)!==0,f||r?(f=n.stateNode,a=r&&typeof a.getDerivedStateFromError!="function"?null:f.render(),n.flags|=1,t!==null&&r?(n.child=ys(n,t.child,null,u),n.child=ys(n,null,a,u)):En(t,n,a,u),n.memoizedState=f.state,t=n.child):t=aa(t,n,u),t}function ng(t,n,a,r){return ps(),n.flags|=256,En(t,n,a,r),n.child}var df={dehydrated:null,treeContext:null,retryLane:0,hydrationErrors:null};function hf(t){return{baseLanes:t,cachePool:jp()}}function pf(t,n,a){return t=t!==null?t.childLanes&~a:0,n&&(t|=ti),t}function ig(t,n,a){var r=n.pendingProps,u=!1,f=(n.flags&128)!==0,_;if((_=f)||(_=t!==null&&t.memoizedState===null?!1:(an.current&2)!==0),_&&(u=!0,n.flags&=-129),_=(n.flags&32)!==0,n.flags&=-33,t===null){if(bt){if(u?Pa(n):Ia(),(t=Xt)?(t=f0(t,fi),t=t!==null&&t.data!=="&"?t:null,t!==null&&(n.memoizedState={dehydrated:t,treeContext:wa!==null?{id:Li,overflow:Oi}:null,retryLane:536870912,hydrationErrors:null},a=Ip(t),a.return=n,n.child=a,bn=n,Xt=null)):t=null,t===null)throw Da(n);return Kf(t)?n.lanes=32:n.lanes=536870912,null}var A=r.children;return r=r.fallback,u?(Ia(),u=n.mode,A=Fl({mode:"hidden",children:A},u),r=hs(r,u,a,null),A.return=n,r.return=n,A.sibling=r,n.child=A,r=n.child,r.memoizedState=hf(a),r.childLanes=pf(t,_,a),n.memoizedState=df,_o(null,r)):(Pa(n),mf(n,A))}var B=t.memoizedState;if(B!==null&&(A=B.dehydrated,A!==null)){if(f)n.flags&256?(Pa(n),n.flags&=-257,n=gf(t,n,a)):n.memoizedState!==null?(Ia(),n.child=t.child,n.flags|=128,n=null):(Ia(),A=r.fallback,u=n.mode,r=Fl({mode:"visible",children:r.children},u),A=hs(A,u,a,null),A.flags|=2,r.return=n,A.return=n,r.sibling=A,n.child=r,ys(n,t.child,null,a),r=n.child,r.memoizedState=hf(a),r.childLanes=pf(t,_,a),n.memoizedState=df,n=_o(null,r));else if(Pa(n),Kf(A)){if(_=A.nextSibling&&A.nextSibling.dataset,_)var $=_.dgst;_=$,r=Error(s(419)),r.stack="",r.digest=_,ao({value:r,source:null,stack:null}),n=gf(t,n,a)}else if(fn||$s(t,n,a,!1),_=(a&t.childLanes)!==0,fn||_){if(_=kt,_!==null&&(r=el(_,a),r!==0&&r!==B.retryLane))throw B.retryLane=r,ds(t,r),Xn(_,t,r),uf;Zf(A)||Wl(),n=gf(t,n,a)}else Zf(A)?(n.flags|=192,n.child=t.child,n=null):(t=B.treeContext,Xt=hi(A.nextSibling),bn=n,bt=!0,Ca=null,fi=!1,t!==null&&Bp(n,t),n=mf(n,r.children),n.flags|=4096);return n}return u?(Ia(),A=r.fallback,u=n.mode,B=t.child,$=B.sibling,r=Ji(B,{mode:"hidden",children:r.children}),r.subtreeFlags=B.subtreeFlags&65011712,$!==null?A=Ji($,A):(A=hs(A,u,a,null),A.flags|=2),A.return=n,r.return=n,r.sibling=A,n.child=r,_o(null,r),r=n.child,A=t.child.memoizedState,A===null?A=hf(a):(u=A.cachePool,u!==null?(B=cn._currentValue,u=u.parent!==B?{parent:B,pool:B}:u):u=jp(),A={baseLanes:A.baseLanes|a,cachePool:u}),r.memoizedState=A,r.childLanes=pf(t,_,a),n.memoizedState=df,_o(t.child,r)):(Pa(n),a=t.child,t=a.sibling,a=Ji(a,{mode:"visible",children:r.children}),a.return=n,a.sibling=null,t!==null&&(_=n.deletions,_===null?(n.deletions=[t],n.flags|=16):_.push(t)),n.child=a,n.memoizedState=null,a)}function mf(t,n){return n=Fl({mode:"visible",children:n},t.mode),n.return=t,t.child=n}function Fl(t,n){return t=Qn(22,t,null,n),t.lanes=0,t}function gf(t,n,a){return ys(n,t.child,null,a),t=mf(n,n.pendingProps.children),t.flags|=2,n.memoizedState=null,t}function ag(t,n,a){t.lanes|=n;var r=t.alternate;r!==null&&(r.lanes|=n),Du(t.return,n,a)}function _f(t,n,a,r,u,f){var _=t.memoizedState;_===null?t.memoizedState={isBackwards:n,rendering:null,renderingStartTime:0,last:r,tail:a,tailMode:u,treeForkCount:f}:(_.isBackwards=n,_.rendering=null,_.renderingStartTime=0,_.last=r,_.tail=a,_.tailMode=u,_.treeForkCount=f)}function sg(t,n,a){var r=n.pendingProps,u=r.revealOrder,f=r.tail;r=r.children;var _=an.current,A=(_&2)!==0;if(A?(_=_&1|2,n.flags|=128):_&=1,ve(an,_),En(t,n,r,a),r=bt?io:0,!A&&t!==null&&(t.flags&128)!==0)e:for(t=n.child;t!==null;){if(t.tag===13)t.memoizedState!==null&&ag(t,a,n);else if(t.tag===19)ag(t,a,n);else if(t.child!==null){t.child.return=t,t=t.child;continue}if(t===n)break e;for(;t.sibling===null;){if(t.return===null||t.return===n)break e;t=t.return}t.sibling.return=t.return,t=t.sibling}switch(u){case"forwards":for(a=n.child,u=null;a!==null;)t=a.alternate,t!==null&&Tl(t)===null&&(u=a),a=a.sibling;a=u,a===null?(u=n.child,n.child=null):(u=a.sibling,a.sibling=null),_f(n,!1,u,a,f,r);break;case"backwards":case"unstable_legacy-backwards":for(a=null,u=n.child,n.child=null;u!==null;){if(t=u.alternate,t!==null&&Tl(t)===null){n.child=u;break}t=u.sibling,u.sibling=a,a=u,u=t}_f(n,!0,a,null,f,r);break;case"together":_f(n,!1,null,null,void 0,r);break;default:n.memoizedState=null}return n.child}function aa(t,n,a){if(t!==null&&(n.dependencies=t.dependencies),Ba|=n.lanes,(a&n.childLanes)===0)if(t!==null){if($s(t,n,a,!1),(a&n.childLanes)===0)return null}else return null;if(t!==null&&n.child!==t.child)throw Error(s(153));if(n.child!==null){for(t=n.child,a=Ji(t,t.pendingProps),n.child=a,a.return=n;t.sibling!==null;)t=t.sibling,a=a.sibling=Ji(t,t.pendingProps),a.return=n;a.sibling=null}return n.child}function vf(t,n){return(t.lanes&n)!==0?!0:(t=t.dependencies,!!(t!==null&&_l(t)))}function Bx(t,n,a){switch(n.tag){case 3:Te(n,n.stateNode.containerInfo),Na(n,cn,t.memoizedState.cache),ps();break;case 27:case 5:Ke(n);break;case 4:Te(n,n.stateNode.containerInfo);break;case 10:Na(n,n.type,n.memoizedProps.value);break;case 31:if(n.memoizedState!==null)return n.flags|=128,Vu(n),null;break;case 13:var r=n.memoizedState;if(r!==null)return r.dehydrated!==null?(Pa(n),n.flags|=128,null):(a&n.child.childLanes)!==0?ig(t,n,a):(Pa(n),t=aa(t,n,a),t!==null?t.sibling:null);Pa(n);break;case 19:var u=(t.flags&128)!==0;if(r=(a&n.childLanes)!==0,r||($s(t,n,a,!1),r=(a&n.childLanes)!==0),u){if(r)return sg(t,n,a);n.flags|=128}if(u=n.memoizedState,u!==null&&(u.rendering=null,u.tail=null,u.lastEffect=null),ve(an,an.current),r)break;return null;case 22:return n.lanes=0,Qm(t,n,a,n.pendingProps);case 24:Na(n,cn,t.memoizedState.cache)}return aa(t,n,a)}function rg(t,n,a){if(t!==null)if(t.memoizedProps!==n.pendingProps)fn=!0;else{if(!vf(t,a)&&(n.flags&128)===0)return fn=!1,Bx(t,n,a);fn=(t.flags&131072)!==0}else fn=!1,bt&&(n.flags&1048576)!==0&&zp(n,io,n.index);switch(n.lanes=0,n.tag){case 16:e:{var r=n.pendingProps;if(t=vs(n.elementType),n.type=t,typeof t=="function")bu(t)?(r=bs(t,r),n.tag=1,n=tg(null,n,t,r,a)):(n.tag=0,n=ff(null,n,t,r,a));else{if(t!=null){var u=t.$$typeof;if(u===N){n.tag=11,n=Ym(null,n,t,r,a);break e}else if(u===F){n.tag=14,n=Zm(null,n,t,r,a);break e}}throw n=ue(t)||t,Error(s(306,n,""))}}return n;case 0:return ff(t,n,n.type,n.pendingProps,a);case 1:return r=n.type,u=bs(r,n.pendingProps),tg(t,n,r,u,a);case 3:e:{if(Te(n,n.stateNode.containerInfo),t===null)throw Error(s(387));r=n.pendingProps;var f=n.memoizedState;u=f.element,Fu(t,n),fo(n,r,null,a);var _=n.memoizedState;if(r=_.cache,Na(n,cn,r),r!==f.cache&&Nu(n,[cn],a,!0),uo(),r=_.element,f.isDehydrated)if(f={element:r,isDehydrated:!1,cache:_.cache},n.updateQueue.baseState=f,n.memoizedState=f,n.flags&256){n=ng(t,n,r,a);break e}else if(r!==u){u=li(Error(s(424)),n),ao(u),n=ng(t,n,r,a);break e}else{switch(t=n.stateNode.containerInfo,t.nodeType){case 9:t=t.body;break;default:t=t.nodeName==="HTML"?t.ownerDocument.body:t}for(Xt=hi(t.firstChild),bn=n,bt=!0,Ca=null,fi=!0,a=Qp(n,null,r,a),n.child=a;a;)a.flags=a.flags&-3|4096,a=a.sibling}else{if(ps(),r===u){n=aa(t,n,a);break e}En(t,n,r,a)}n=n.child}return n;case 26:return Il(t,n),t===null?(a=_0(n.type,null,n.pendingProps,null))?n.memoizedState=a:bt||(a=n.type,t=n.pendingProps,r=$l(ie.current).createElement(a),r[rn]=n,r[pn]=t,Tn(r,a,t),j(r),n.stateNode=r):n.memoizedState=_0(n.type,t.memoizedProps,n.pendingProps,t.memoizedState),null;case 27:return Ke(n),t===null&&bt&&(r=n.stateNode=p0(n.type,n.pendingProps,ie.current),bn=n,fi=!0,u=Xt,Xa(n.type)?(Qf=u,Xt=hi(r.firstChild)):Xt=u),En(t,n,n.pendingProps.children,a),Il(t,n),t===null&&(n.flags|=4194304),n.child;case 5:return t===null&&bt&&((u=r=Xt)&&(r=my(r,n.type,n.pendingProps,fi),r!==null?(n.stateNode=r,bn=n,Xt=hi(r.firstChild),fi=!1,u=!0):u=!1),u||Da(n)),Ke(n),u=n.type,f=n.pendingProps,_=t!==null?t.memoizedProps:null,r=f.children,Wf(u,f)?r=null:_!==null&&Wf(u,_)&&(n.flags|=32),n.memoizedState!==null&&(u=Xu(t,n,Dx,null,null,a),Uo._currentValue=u),Il(t,n),En(t,n,r,a),n.child;case 6:return t===null&&bt&&((t=a=Xt)&&(a=gy(a,n.pendingProps,fi),a!==null?(n.stateNode=a,bn=n,Xt=null,t=!0):t=!1),t||Da(n)),null;case 13:return ig(t,n,a);case 4:return Te(n,n.stateNode.containerInfo),r=n.pendingProps,t===null?n.child=ys(n,null,r,a):En(t,n,r,a),n.child;case 11:return Ym(t,n,n.type,n.pendingProps,a);case 7:return En(t,n,n.pendingProps,a),n.child;case 8:return En(t,n,n.pendingProps.children,a),n.child;case 12:return En(t,n,n.pendingProps.children,a),n.child;case 10:return r=n.pendingProps,Na(n,n.type,r.value),En(t,n,r.children,a),n.child;case 9:return u=n.type._context,r=n.pendingProps.children,gs(n),u=Mn(u),r=r(u),n.flags|=1,En(t,n,r,a),n.child;case 14:return Zm(t,n,n.type,n.pendingProps,a);case 15:return Km(t,n,n.type,n.pendingProps,a);case 19:return sg(t,n,a);case 31:return zx(t,n,a);case 22:return Qm(t,n,a,n.pendingProps);case 24:return gs(n),r=Mn(cn),t===null?(u=Ou(),u===null&&(u=kt,f=Uu(),u.pooledCache=f,f.refCount++,f!==null&&(u.pooledCacheLanes|=a),u=f),n.memoizedState={parent:r,cache:u},Iu(n),Na(n,cn,u)):((t.lanes&a)!==0&&(Fu(t,n),fo(n,null,null,a),uo()),u=t.memoizedState,f=n.memoizedState,u.parent!==r?(u={parent:r,cache:r},n.memoizedState=u,n.lanes===0&&(n.memoizedState=n.updateQueue.baseState=u),Na(n,cn,r)):(r=f.cache,Na(n,cn,r),r!==u.cache&&Nu(n,[cn],a,!0))),En(t,n,n.pendingProps.children,a),n.child;case 29:throw n.pendingProps}throw Error(s(156,n.tag))}function sa(t){t.flags|=4}function xf(t,n,a,r,u){if((n=(t.mode&32)!==0)&&(n=!1),n){if(t.flags|=16777216,(u&335544128)===u)if(t.stateNode.complete)t.flags|=8192;else if(Lg())t.flags|=8192;else throw xs=Sl,Pu}else t.flags&=-16777217}function og(t,n){if(n.type!=="stylesheet"||(n.state.loading&4)!==0)t.flags&=-16777217;else if(t.flags|=16777216,!b0(n))if(Lg())t.flags|=8192;else throw xs=Sl,Pu}function zl(t,n){n!==null&&(t.flags|=4),t.flags&16384&&(n=t.tag!==22?Pt():536870912,t.lanes|=n,fr|=n)}function vo(t,n){if(!bt)switch(t.tailMode){case"hidden":n=t.tail;for(var a=null;n!==null;)n.alternate!==null&&(a=n),n=n.sibling;a===null?t.tail=null:a.sibling=null;break;case"collapsed":a=t.tail;for(var r=null;a!==null;)a.alternate!==null&&(r=a),a=a.sibling;r===null?n||t.tail===null?t.tail=null:t.tail.sibling=null:r.sibling=null}}function jt(t){var n=t.alternate!==null&&t.alternate.child===t.child,a=0,r=0;if(n)for(var u=t.child;u!==null;)a|=u.lanes|u.childLanes,r|=u.subtreeFlags&65011712,r|=u.flags&65011712,u.return=t,u=u.sibling;else for(u=t.child;u!==null;)a|=u.lanes|u.childLanes,r|=u.subtreeFlags,r|=u.flags,u.return=t,u=u.sibling;return t.subtreeFlags|=r,t.childLanes=a,n}function Hx(t,n,a){var r=n.pendingProps;switch(Au(n),n.tag){case 16:case 15:case 0:case 11:case 7:case 8:case 12:case 9:case 14:return jt(n),null;case 1:return jt(n),null;case 3:return a=n.stateNode,r=null,t!==null&&(r=t.memoizedState.cache),n.memoizedState.cache!==r&&(n.flags|=2048),ta(cn),ke(),a.pendingContext&&(a.context=a.pendingContext,a.pendingContext=null),(t===null||t.child===null)&&(Js(n)?sa(n):t===null||t.memoizedState.isDehydrated&&(n.flags&256)===0||(n.flags|=1024,wu())),jt(n),null;case 26:var u=n.type,f=n.memoizedState;return t===null?(sa(n),f!==null?(jt(n),og(n,f)):(jt(n),xf(n,u,null,r,a))):f?f!==t.memoizedState?(sa(n),jt(n),og(n,f)):(jt(n),n.flags&=-16777217):(t=t.memoizedProps,t!==r&&sa(n),jt(n),xf(n,u,t,r,a)),null;case 27:if($e(n),a=ie.current,u=n.type,t!==null&&n.stateNode!=null)t.memoizedProps!==r&&sa(n);else{if(!r){if(n.stateNode===null)throw Error(s(166));return jt(n),null}t=Re.current,Js(n)?Hp(n):(t=p0(u,r,a),n.stateNode=t,sa(n))}return jt(n),null;case 5:if($e(n),u=n.type,t!==null&&n.stateNode!=null)t.memoizedProps!==r&&sa(n);else{if(!r){if(n.stateNode===null)throw Error(s(166));return jt(n),null}if(f=Re.current,Js(n))Hp(n);else{var _=$l(ie.current);switch(f){case 1:f=_.createElementNS("http://www.w3.org/2000/svg",u);break;case 2:f=_.createElementNS("http://www.w3.org/1998/Math/MathML",u);break;default:switch(u){case"svg":f=_.createElementNS("http://www.w3.org/2000/svg",u);break;case"math":f=_.createElementNS("http://www.w3.org/1998/Math/MathML",u);break;case"script":f=_.createElement("div"),f.innerHTML="<script><\/script>",f=f.removeChild(f.firstChild);break;case"select":f=typeof r.is=="string"?_.createElement("select",{is:r.is}):_.createElement("select"),r.multiple?f.multiple=!0:r.size&&(f.size=r.size);break;default:f=typeof r.is=="string"?_.createElement(u,{is:r.is}):_.createElement(u)}}f[rn]=n,f[pn]=r;e:for(_=n.child;_!==null;){if(_.tag===5||_.tag===6)f.appendChild(_.stateNode);else if(_.tag!==4&&_.tag!==27&&_.child!==null){_.child.return=_,_=_.child;continue}if(_===n)break e;for(;_.sibling===null;){if(_.return===null||_.return===n)break e;_=_.return}_.sibling.return=_.return,_=_.sibling}n.stateNode=f;e:switch(Tn(f,u,r),u){case"button":case"input":case"select":case"textarea":r=!!r.autoFocus;break e;case"img":r=!0;break e;default:r=!1}r&&sa(n)}}return jt(n),xf(n,n.type,t===null?null:t.memoizedProps,n.pendingProps,a),null;case 6:if(t&&n.stateNode!=null)t.memoizedProps!==r&&sa(n);else{if(typeof r!="string"&&n.stateNode===null)throw Error(s(166));if(t=ie.current,Js(n)){if(t=n.stateNode,a=n.memoizedProps,r=null,u=bn,u!==null)switch(u.tag){case 27:case 5:r=u.memoizedProps}t[rn]=n,t=!!(t.nodeValue===a||r!==null&&r.suppressHydrationWarning===!0||i0(t.nodeValue,a)),t||Da(n,!0)}else t=$l(t).createTextNode(r),t[rn]=n,n.stateNode=t}return jt(n),null;case 31:if(a=n.memoizedState,t===null||t.memoizedState!==null){if(r=Js(n),a!==null){if(t===null){if(!r)throw Error(s(318));if(t=n.memoizedState,t=t!==null?t.dehydrated:null,!t)throw Error(s(557));t[rn]=n}else ps(),(n.flags&128)===0&&(n.memoizedState=null),n.flags|=4;jt(n),t=!1}else a=wu(),t!==null&&t.memoizedState!==null&&(t.memoizedState.hydrationErrors=a),t=!0;if(!t)return n.flags&256?($n(n),n):($n(n),null);if((n.flags&128)!==0)throw Error(s(558))}return jt(n),null;case 13:if(r=n.memoizedState,t===null||t.memoizedState!==null&&t.memoizedState.dehydrated!==null){if(u=Js(n),r!==null&&r.dehydrated!==null){if(t===null){if(!u)throw Error(s(318));if(u=n.memoizedState,u=u!==null?u.dehydrated:null,!u)throw Error(s(317));u[rn]=n}else ps(),(n.flags&128)===0&&(n.memoizedState=null),n.flags|=4;jt(n),u=!1}else u=wu(),t!==null&&t.memoizedState!==null&&(t.memoizedState.hydrationErrors=u),u=!0;if(!u)return n.flags&256?($n(n),n):($n(n),null)}return $n(n),(n.flags&128)!==0?(n.lanes=a,n):(a=r!==null,t=t!==null&&t.memoizedState!==null,a&&(r=n.child,u=null,r.alternate!==null&&r.alternate.memoizedState!==null&&r.alternate.memoizedState.cachePool!==null&&(u=r.alternate.memoizedState.cachePool.pool),f=null,r.memoizedState!==null&&r.memoizedState.cachePool!==null&&(f=r.memoizedState.cachePool.pool),f!==u&&(r.flags|=2048)),a!==t&&a&&(n.child.flags|=8192),zl(n,n.updateQueue),jt(n),null);case 4:return ke(),t===null&&Gf(n.stateNode.containerInfo),jt(n),null;case 10:return ta(n.type),jt(n),null;case 19:if(Y(an),r=n.memoizedState,r===null)return jt(n),null;if(u=(n.flags&128)!==0,f=r.rendering,f===null)if(u)vo(r,!1);else{if(tn!==0||t!==null&&(t.flags&128)!==0)for(t=n.child;t!==null;){if(f=Tl(t),f!==null){for(n.flags|=128,vo(r,!1),t=f.updateQueue,n.updateQueue=t,zl(n,t),n.subtreeFlags=0,t=a,a=n.child;a!==null;)Pp(a,t),a=a.sibling;return ve(an,an.current&1|2),bt&&$i(n,r.treeForkCount),n.child}t=t.sibling}r.tail!==null&&M()>kl&&(n.flags|=128,u=!0,vo(r,!1),n.lanes=4194304)}else{if(!u)if(t=Tl(f),t!==null){if(n.flags|=128,u=!0,t=t.updateQueue,n.updateQueue=t,zl(n,t),vo(r,!0),r.tail===null&&r.tailMode==="hidden"&&!f.alternate&&!bt)return jt(n),null}else 2*M()-r.renderingStartTime>kl&&a!==536870912&&(n.flags|=128,u=!0,vo(r,!1),n.lanes=4194304);r.isBackwards?(f.sibling=n.child,n.child=f):(t=r.last,t!==null?t.sibling=f:n.child=f,r.last=f)}return r.tail!==null?(t=r.tail,r.rendering=t,r.tail=t.sibling,r.renderingStartTime=M(),t.sibling=null,a=an.current,ve(an,u?a&1|2:a&1),bt&&$i(n,r.treeForkCount),t):(jt(n),null);case 22:case 23:return $n(n),Gu(),r=n.memoizedState!==null,t!==null?t.memoizedState!==null!==r&&(n.flags|=8192):r&&(n.flags|=8192),r?(a&536870912)!==0&&(n.flags&128)===0&&(jt(n),n.subtreeFlags&6&&(n.flags|=8192)):jt(n),a=n.updateQueue,a!==null&&zl(n,a.retryQueue),a=null,t!==null&&t.memoizedState!==null&&t.memoizedState.cachePool!==null&&(a=t.memoizedState.cachePool.pool),r=null,n.memoizedState!==null&&n.memoizedState.cachePool!==null&&(r=n.memoizedState.cachePool.pool),r!==a&&(n.flags|=2048),t!==null&&Y(_s),null;case 24:return a=null,t!==null&&(a=t.memoizedState.cache),n.memoizedState.cache!==a&&(n.flags|=2048),ta(cn),jt(n),null;case 25:return null;case 30:return null}throw Error(s(156,n.tag))}function Gx(t,n){switch(Au(n),n.tag){case 1:return t=n.flags,t&65536?(n.flags=t&-65537|128,n):null;case 3:return ta(cn),ke(),t=n.flags,(t&65536)!==0&&(t&128)===0?(n.flags=t&-65537|128,n):null;case 26:case 27:case 5:return $e(n),null;case 31:if(n.memoizedState!==null){if($n(n),n.alternate===null)throw Error(s(340));ps()}return t=n.flags,t&65536?(n.flags=t&-65537|128,n):null;case 13:if($n(n),t=n.memoizedState,t!==null&&t.dehydrated!==null){if(n.alternate===null)throw Error(s(340));ps()}return t=n.flags,t&65536?(n.flags=t&-65537|128,n):null;case 19:return Y(an),null;case 4:return ke(),null;case 10:return ta(n.type),null;case 22:case 23:return $n(n),Gu(),t!==null&&Y(_s),t=n.flags,t&65536?(n.flags=t&-65537|128,n):null;case 24:return ta(cn),null;case 25:return null;default:return null}}function lg(t,n){switch(Au(n),n.tag){case 3:ta(cn),ke();break;case 26:case 27:case 5:$e(n);break;case 4:ke();break;case 31:n.memoizedState!==null&&$n(n);break;case 13:$n(n);break;case 19:Y(an);break;case 10:ta(n.type);break;case 22:case 23:$n(n),Gu(),t!==null&&Y(_s);break;case 24:ta(cn)}}function xo(t,n){try{var a=n.updateQueue,r=a!==null?a.lastEffect:null;if(r!==null){var u=r.next;a=u;do{if((a.tag&t)===t){r=void 0;var f=a.create,_=a.inst;r=f(),_.destroy=r}a=a.next}while(a!==u)}}catch(A){Ft(n,n.return,A)}}function Fa(t,n,a){try{var r=n.updateQueue,u=r!==null?r.lastEffect:null;if(u!==null){var f=u.next;r=f;do{if((r.tag&t)===t){var _=r.inst,A=_.destroy;if(A!==void 0){_.destroy=void 0,u=n;var B=a,$=A;try{$()}catch(he){Ft(u,B,he)}}}r=r.next}while(r!==f)}}catch(he){Ft(n,n.return,he)}}function cg(t){var n=t.updateQueue;if(n!==null){var a=t.stateNode;try{$p(n,a)}catch(r){Ft(t,t.return,r)}}}function ug(t,n,a){a.props=bs(t.type,t.memoizedProps),a.state=t.memoizedState;try{a.componentWillUnmount()}catch(r){Ft(t,n,r)}}function yo(t,n){try{var a=t.ref;if(a!==null){switch(t.tag){case 26:case 27:case 5:var r=t.stateNode;break;case 30:r=t.stateNode;break;default:r=t.stateNode}typeof a=="function"?t.refCleanup=a(r):a.current=r}}catch(u){Ft(t,n,u)}}function Pi(t,n){var a=t.ref,r=t.refCleanup;if(a!==null)if(typeof r=="function")try{r()}catch(u){Ft(t,n,u)}finally{t.refCleanup=null,t=t.alternate,t!=null&&(t.refCleanup=null)}else if(typeof a=="function")try{a(null)}catch(u){Ft(t,n,u)}else a.current=null}function fg(t){var n=t.type,a=t.memoizedProps,r=t.stateNode;try{e:switch(n){case"button":case"input":case"select":case"textarea":a.autoFocus&&r.focus();break e;case"img":a.src?r.src=a.src:a.srcSet&&(r.srcset=a.srcSet)}}catch(u){Ft(t,t.return,u)}}function yf(t,n,a){try{var r=t.stateNode;cy(r,t.type,a,n),r[pn]=n}catch(u){Ft(t,t.return,u)}}function dg(t){return t.tag===5||t.tag===3||t.tag===26||t.tag===27&&Xa(t.type)||t.tag===4}function Sf(t){e:for(;;){for(;t.sibling===null;){if(t.return===null||dg(t.return))return null;t=t.return}for(t.sibling.return=t.return,t=t.sibling;t.tag!==5&&t.tag!==6&&t.tag!==18;){if(t.tag===27&&Xa(t.type)||t.flags&2||t.child===null||t.tag===4)continue e;t.child.return=t,t=t.child}if(!(t.flags&2))return t.stateNode}}function bf(t,n,a){var r=t.tag;if(r===5||r===6)t=t.stateNode,n?(a.nodeType===9?a.body:a.nodeName==="HTML"?a.ownerDocument.body:a).insertBefore(t,n):(n=a.nodeType===9?a.body:a.nodeName==="HTML"?a.ownerDocument.body:a,n.appendChild(t),a=a._reactRootContainer,a!=null||n.onclick!==null||(n.onclick=Ki));else if(r!==4&&(r===27&&Xa(t.type)&&(a=t.stateNode,n=null),t=t.child,t!==null))for(bf(t,n,a),t=t.sibling;t!==null;)bf(t,n,a),t=t.sibling}function Bl(t,n,a){var r=t.tag;if(r===5||r===6)t=t.stateNode,n?a.insertBefore(t,n):a.appendChild(t);else if(r!==4&&(r===27&&Xa(t.type)&&(a=t.stateNode),t=t.child,t!==null))for(Bl(t,n,a),t=t.sibling;t!==null;)Bl(t,n,a),t=t.sibling}function hg(t){var n=t.stateNode,a=t.memoizedProps;try{for(var r=t.type,u=n.attributes;u.length;)n.removeAttributeNode(u[0]);Tn(n,r,a),n[rn]=t,n[pn]=a}catch(f){Ft(t,t.return,f)}}var ra=!1,dn=!1,Mf=!1,pg=typeof WeakSet=="function"?WeakSet:Set,yn=null;function Vx(t,n){if(t=t.containerInfo,Xf=rc,t=Ap(t),mu(t)){if("selectionStart"in t)var a={start:t.selectionStart,end:t.selectionEnd};else e:{a=(a=t.ownerDocument)&&a.defaultView||window;var r=a.getSelection&&a.getSelection();if(r&&r.rangeCount!==0){a=r.anchorNode;var u=r.anchorOffset,f=r.focusNode;r=r.focusOffset;try{a.nodeType,f.nodeType}catch{a=null;break e}var _=0,A=-1,B=-1,$=0,he=0,_e=t,ae=null;t:for(;;){for(var oe;_e!==a||u!==0&&_e.nodeType!==3||(A=_+u),_e!==f||r!==0&&_e.nodeType!==3||(B=_+r),_e.nodeType===3&&(_+=_e.nodeValue.length),(oe=_e.firstChild)!==null;)ae=_e,_e=oe;for(;;){if(_e===t)break t;if(ae===a&&++$===u&&(A=_),ae===f&&++he===r&&(B=_),(oe=_e.nextSibling)!==null)break;_e=ae,ae=_e.parentNode}_e=oe}a=A===-1||B===-1?null:{start:A,end:B}}else a=null}a=a||{start:0,end:0}}else a=null;for(jf={focusedElem:t,selectionRange:a},rc=!1,yn=n;yn!==null;)if(n=yn,t=n.child,(n.subtreeFlags&1028)!==0&&t!==null)t.return=n,yn=t;else for(;yn!==null;){switch(n=yn,f=n.alternate,t=n.flags,n.tag){case 0:if((t&4)!==0&&(t=n.updateQueue,t=t!==null?t.events:null,t!==null))for(a=0;a<t.length;a++)u=t[a],u.ref.impl=u.nextImpl;break;case 11:case 15:break;case 1:if((t&1024)!==0&&f!==null){t=void 0,a=n,u=f.memoizedProps,f=f.memoizedState,r=a.stateNode;try{var Ge=bs(a.type,u);t=r.getSnapshotBeforeUpdate(Ge,f),r.__reactInternalSnapshotBeforeUpdate=t}catch(Je){Ft(a,a.return,Je)}}break;case 3:if((t&1024)!==0){if(t=n.stateNode.containerInfo,a=t.nodeType,a===9)Yf(t);else if(a===1)switch(t.nodeName){case"HEAD":case"HTML":case"BODY":Yf(t);break;default:t.textContent=""}}break;case 5:case 26:case 27:case 6:case 4:case 17:break;default:if((t&1024)!==0)throw Error(s(163))}if(t=n.sibling,t!==null){t.return=n.return,yn=t;break}yn=n.return}}function mg(t,n,a){var r=a.flags;switch(a.tag){case 0:case 11:case 15:la(t,a),r&4&&xo(5,a);break;case 1:if(la(t,a),r&4)if(t=a.stateNode,n===null)try{t.componentDidMount()}catch(_){Ft(a,a.return,_)}else{var u=bs(a.type,n.memoizedProps);n=n.memoizedState;try{t.componentDidUpdate(u,n,t.__reactInternalSnapshotBeforeUpdate)}catch(_){Ft(a,a.return,_)}}r&64&&cg(a),r&512&&yo(a,a.return);break;case 3:if(la(t,a),r&64&&(t=a.updateQueue,t!==null)){if(n=null,a.child!==null)switch(a.child.tag){case 27:case 5:n=a.child.stateNode;break;case 1:n=a.child.stateNode}try{$p(t,n)}catch(_){Ft(a,a.return,_)}}break;case 27:n===null&&r&4&&hg(a);case 26:case 5:la(t,a),n===null&&r&4&&fg(a),r&512&&yo(a,a.return);break;case 12:la(t,a);break;case 31:la(t,a),r&4&&vg(t,a);break;case 13:la(t,a),r&4&&xg(t,a),r&64&&(t=a.memoizedState,t!==null&&(t=t.dehydrated,t!==null&&(a=Qx.bind(null,a),_y(t,a))));break;case 22:if(r=a.memoizedState!==null||ra,!r){n=n!==null&&n.memoizedState!==null||dn,u=ra;var f=dn;ra=r,(dn=n)&&!f?ca(t,a,(a.subtreeFlags&8772)!==0):la(t,a),ra=u,dn=f}break;case 30:break;default:la(t,a)}}function gg(t){var n=t.alternate;n!==null&&(t.alternate=null,gg(n)),t.child=null,t.deletions=null,t.sibling=null,t.tag===5&&(n=t.stateNode,n!==null&&qr(n)),t.stateNode=null,t.return=null,t.dependencies=null,t.memoizedProps=null,t.memoizedState=null,t.pendingProps=null,t.stateNode=null,t.updateQueue=null}var Kt=null,Hn=!1;function oa(t,n,a){for(a=a.child;a!==null;)_g(t,n,a),a=a.sibling}function _g(t,n,a){if(Se&&typeof Se.onCommitFiberUnmount=="function")try{Se.onCommitFiberUnmount(Me,a)}catch{}switch(a.tag){case 26:dn||Pi(a,n),oa(t,n,a),a.memoizedState?a.memoizedState.count--:a.stateNode&&(a=a.stateNode,a.parentNode.removeChild(a));break;case 27:dn||Pi(a,n);var r=Kt,u=Hn;Xa(a.type)&&(Kt=a.stateNode,Hn=!1),oa(t,n,a),Co(a.stateNode),Kt=r,Hn=u;break;case 5:dn||Pi(a,n);case 6:if(r=Kt,u=Hn,Kt=null,oa(t,n,a),Kt=r,Hn=u,Kt!==null)if(Hn)try{(Kt.nodeType===9?Kt.body:Kt.nodeName==="HTML"?Kt.ownerDocument.body:Kt).removeChild(a.stateNode)}catch(f){Ft(a,n,f)}else try{Kt.removeChild(a.stateNode)}catch(f){Ft(a,n,f)}break;case 18:Kt!==null&&(Hn?(t=Kt,c0(t.nodeType===9?t.body:t.nodeName==="HTML"?t.ownerDocument.body:t,a.stateNode),xr(t)):c0(Kt,a.stateNode));break;case 4:r=Kt,u=Hn,Kt=a.stateNode.containerInfo,Hn=!0,oa(t,n,a),Kt=r,Hn=u;break;case 0:case 11:case 14:case 15:Fa(2,a,n),dn||Fa(4,a,n),oa(t,n,a);break;case 1:dn||(Pi(a,n),r=a.stateNode,typeof r.componentWillUnmount=="function"&&ug(a,n,r)),oa(t,n,a);break;case 21:oa(t,n,a);break;case 22:dn=(r=dn)||a.memoizedState!==null,oa(t,n,a),dn=r;break;default:oa(t,n,a)}}function vg(t,n){if(n.memoizedState===null&&(t=n.alternate,t!==null&&(t=t.memoizedState,t!==null))){t=t.dehydrated;try{xr(t)}catch(a){Ft(n,n.return,a)}}}function xg(t,n){if(n.memoizedState===null&&(t=n.alternate,t!==null&&(t=t.memoizedState,t!==null&&(t=t.dehydrated,t!==null))))try{xr(t)}catch(a){Ft(n,n.return,a)}}function kx(t){switch(t.tag){case 31:case 13:case 19:var n=t.stateNode;return n===null&&(n=t.stateNode=new pg),n;case 22:return t=t.stateNode,n=t._retryCache,n===null&&(n=t._retryCache=new pg),n;default:throw Error(s(435,t.tag))}}function Hl(t,n){var a=kx(t);n.forEach(function(r){if(!a.has(r)){a.add(r);var u=Jx.bind(null,t,r);r.then(u,u)}})}function Gn(t,n){var a=n.deletions;if(a!==null)for(var r=0;r<a.length;r++){var u=a[r],f=t,_=n,A=_;e:for(;A!==null;){switch(A.tag){case 27:if(Xa(A.type)){Kt=A.stateNode,Hn=!1;break e}break;case 5:Kt=A.stateNode,Hn=!1;break e;case 3:case 4:Kt=A.stateNode.containerInfo,Hn=!0;break e}A=A.return}if(Kt===null)throw Error(s(160));_g(f,_,u),Kt=null,Hn=!1,f=u.alternate,f!==null&&(f.return=null),u.return=null}if(n.subtreeFlags&13886)for(n=n.child;n!==null;)yg(n,t),n=n.sibling}var Ei=null;function yg(t,n){var a=t.alternate,r=t.flags;switch(t.tag){case 0:case 11:case 14:case 15:Gn(n,t),Vn(t),r&4&&(Fa(3,t,t.return),xo(3,t),Fa(5,t,t.return));break;case 1:Gn(n,t),Vn(t),r&512&&(dn||a===null||Pi(a,a.return)),r&64&&ra&&(t=t.updateQueue,t!==null&&(r=t.callbacks,r!==null&&(a=t.shared.hiddenCallbacks,t.shared.hiddenCallbacks=a===null?r:a.concat(r))));break;case 26:var u=Ei;if(Gn(n,t),Vn(t),r&512&&(dn||a===null||Pi(a,a.return)),r&4){var f=a!==null?a.memoizedState:null;if(r=t.memoizedState,a===null)if(r===null)if(t.stateNode===null){e:{r=t.type,a=t.memoizedProps,u=u.ownerDocument||u;t:switch(r){case"title":f=u.getElementsByTagName("title")[0],(!f||f[os]||f[rn]||f.namespaceURI==="http://www.w3.org/2000/svg"||f.hasAttribute("itemprop"))&&(f=u.createElement(r),u.head.insertBefore(f,u.querySelector("head > title"))),Tn(f,r,a),f[rn]=t,j(f),r=f;break e;case"link":var _=y0("link","href",u).get(r+(a.href||""));if(_){for(var A=0;A<_.length;A++)if(f=_[A],f.getAttribute("href")===(a.href==null||a.href===""?null:a.href)&&f.getAttribute("rel")===(a.rel==null?null:a.rel)&&f.getAttribute("title")===(a.title==null?null:a.title)&&f.getAttribute("crossorigin")===(a.crossOrigin==null?null:a.crossOrigin)){_.splice(A,1);break t}}f=u.createElement(r),Tn(f,r,a),u.head.appendChild(f);break;case"meta":if(_=y0("meta","content",u).get(r+(a.content||""))){for(A=0;A<_.length;A++)if(f=_[A],f.getAttribute("content")===(a.content==null?null:""+a.content)&&f.getAttribute("name")===(a.name==null?null:a.name)&&f.getAttribute("property")===(a.property==null?null:a.property)&&f.getAttribute("http-equiv")===(a.httpEquiv==null?null:a.httpEquiv)&&f.getAttribute("charset")===(a.charSet==null?null:a.charSet)){_.splice(A,1);break t}}f=u.createElement(r),Tn(f,r,a),u.head.appendChild(f);break;default:throw Error(s(468,r))}f[rn]=t,j(f),r=f}t.stateNode=r}else S0(u,t.type,t.stateNode);else t.stateNode=x0(u,r,t.memoizedProps);else f!==r?(f===null?a.stateNode!==null&&(a=a.stateNode,a.parentNode.removeChild(a)):f.count--,r===null?S0(u,t.type,t.stateNode):x0(u,r,t.memoizedProps)):r===null&&t.stateNode!==null&&yf(t,t.memoizedProps,a.memoizedProps)}break;case 27:Gn(n,t),Vn(t),r&512&&(dn||a===null||Pi(a,a.return)),a!==null&&r&4&&yf(t,t.memoizedProps,a.memoizedProps);break;case 5:if(Gn(n,t),Vn(t),r&512&&(dn||a===null||Pi(a,a.return)),t.flags&32){u=t.stateNode;try{On(u,"")}catch(Ge){Ft(t,t.return,Ge)}}r&4&&t.stateNode!=null&&(u=t.memoizedProps,yf(t,u,a!==null?a.memoizedProps:u)),r&1024&&(Mf=!0);break;case 6:if(Gn(n,t),Vn(t),r&4){if(t.stateNode===null)throw Error(s(162));r=t.memoizedProps,a=t.stateNode;try{a.nodeValue=r}catch(Ge){Ft(t,t.return,Ge)}}break;case 3:if(nc=null,u=Ei,Ei=ec(n.containerInfo),Gn(n,t),Ei=u,Vn(t),r&4&&a!==null&&a.memoizedState.isDehydrated)try{xr(n.containerInfo)}catch(Ge){Ft(t,t.return,Ge)}Mf&&(Mf=!1,Sg(t));break;case 4:r=Ei,Ei=ec(t.stateNode.containerInfo),Gn(n,t),Vn(t),Ei=r;break;case 12:Gn(n,t),Vn(t);break;case 31:Gn(n,t),Vn(t),r&4&&(r=t.updateQueue,r!==null&&(t.updateQueue=null,Hl(t,r)));break;case 13:Gn(n,t),Vn(t),t.child.flags&8192&&t.memoizedState!==null!=(a!==null&&a.memoizedState!==null)&&(Vl=M()),r&4&&(r=t.updateQueue,r!==null&&(t.updateQueue=null,Hl(t,r)));break;case 22:u=t.memoizedState!==null;var B=a!==null&&a.memoizedState!==null,$=ra,he=dn;if(ra=$||u,dn=he||B,Gn(n,t),dn=he,ra=$,Vn(t),r&8192)e:for(n=t.stateNode,n._visibility=u?n._visibility&-2:n._visibility|1,u&&(a===null||B||ra||dn||Ms(t)),a=null,n=t;;){if(n.tag===5||n.tag===26){if(a===null){B=a=n;try{if(f=B.stateNode,u)_=f.style,typeof _.setProperty=="function"?_.setProperty("display","none","important"):_.display="none";else{A=B.stateNode;var _e=B.memoizedProps.style,ae=_e!=null&&_e.hasOwnProperty("display")?_e.display:null;A.style.display=ae==null||typeof ae=="boolean"?"":(""+ae).trim()}}catch(Ge){Ft(B,B.return,Ge)}}}else if(n.tag===6){if(a===null){B=n;try{B.stateNode.nodeValue=u?"":B.memoizedProps}catch(Ge){Ft(B,B.return,Ge)}}}else if(n.tag===18){if(a===null){B=n;try{var oe=B.stateNode;u?u0(oe,!0):u0(B.stateNode,!1)}catch(Ge){Ft(B,B.return,Ge)}}}else if((n.tag!==22&&n.tag!==23||n.memoizedState===null||n===t)&&n.child!==null){n.child.return=n,n=n.child;continue}if(n===t)break e;for(;n.sibling===null;){if(n.return===null||n.return===t)break e;a===n&&(a=null),n=n.return}a===n&&(a=null),n.sibling.return=n.return,n=n.sibling}r&4&&(r=t.updateQueue,r!==null&&(a=r.retryQueue,a!==null&&(r.retryQueue=null,Hl(t,a))));break;case 19:Gn(n,t),Vn(t),r&4&&(r=t.updateQueue,r!==null&&(t.updateQueue=null,Hl(t,r)));break;case 30:break;case 21:break;default:Gn(n,t),Vn(t)}}function Vn(t){var n=t.flags;if(n&2){try{for(var a,r=t.return;r!==null;){if(dg(r)){a=r;break}r=r.return}if(a==null)throw Error(s(160));switch(a.tag){case 27:var u=a.stateNode,f=Sf(t);Bl(t,f,u);break;case 5:var _=a.stateNode;a.flags&32&&(On(_,""),a.flags&=-33);var A=Sf(t);Bl(t,A,_);break;case 3:case 4:var B=a.stateNode.containerInfo,$=Sf(t);bf(t,$,B);break;default:throw Error(s(161))}}catch(he){Ft(t,t.return,he)}t.flags&=-3}n&4096&&(t.flags&=-4097)}function Sg(t){if(t.subtreeFlags&1024)for(t=t.child;t!==null;){var n=t;Sg(n),n.tag===5&&n.flags&1024&&n.stateNode.reset(),t=t.sibling}}function la(t,n){if(n.subtreeFlags&8772)for(n=n.child;n!==null;)mg(t,n.alternate,n),n=n.sibling}function Ms(t){for(t=t.child;t!==null;){var n=t;switch(n.tag){case 0:case 11:case 14:case 15:Fa(4,n,n.return),Ms(n);break;case 1:Pi(n,n.return);var a=n.stateNode;typeof a.componentWillUnmount=="function"&&ug(n,n.return,a),Ms(n);break;case 27:Co(n.stateNode);case 26:case 5:Pi(n,n.return),Ms(n);break;case 22:n.memoizedState===null&&Ms(n);break;case 30:Ms(n);break;default:Ms(n)}t=t.sibling}}function ca(t,n,a){for(a=a&&(n.subtreeFlags&8772)!==0,n=n.child;n!==null;){var r=n.alternate,u=t,f=n,_=f.flags;switch(f.tag){case 0:case 11:case 15:ca(u,f,a),xo(4,f);break;case 1:if(ca(u,f,a),r=f,u=r.stateNode,typeof u.componentDidMount=="function")try{u.componentDidMount()}catch($){Ft(r,r.return,$)}if(r=f,u=r.updateQueue,u!==null){var A=r.stateNode;try{var B=u.shared.hiddenCallbacks;if(B!==null)for(u.shared.hiddenCallbacks=null,u=0;u<B.length;u++)Jp(B[u],A)}catch($){Ft(r,r.return,$)}}a&&_&64&&cg(f),yo(f,f.return);break;case 27:hg(f);case 26:case 5:ca(u,f,a),a&&r===null&&_&4&&fg(f),yo(f,f.return);break;case 12:ca(u,f,a);break;case 31:ca(u,f,a),a&&_&4&&vg(u,f);break;case 13:ca(u,f,a),a&&_&4&&xg(u,f);break;case 22:f.memoizedState===null&&ca(u,f,a),yo(f,f.return);break;case 30:break;default:ca(u,f,a)}n=n.sibling}}function Ef(t,n){var a=null;t!==null&&t.memoizedState!==null&&t.memoizedState.cachePool!==null&&(a=t.memoizedState.cachePool.pool),t=null,n.memoizedState!==null&&n.memoizedState.cachePool!==null&&(t=n.memoizedState.cachePool.pool),t!==a&&(t!=null&&t.refCount++,a!=null&&so(a))}function Tf(t,n){t=null,n.alternate!==null&&(t=n.alternate.memoizedState.cache),n=n.memoizedState.cache,n!==t&&(n.refCount++,t!=null&&so(t))}function Ti(t,n,a,r){if(n.subtreeFlags&10256)for(n=n.child;n!==null;)bg(t,n,a,r),n=n.sibling}function bg(t,n,a,r){var u=n.flags;switch(n.tag){case 0:case 11:case 15:Ti(t,n,a,r),u&2048&&xo(9,n);break;case 1:Ti(t,n,a,r);break;case 3:Ti(t,n,a,r),u&2048&&(t=null,n.alternate!==null&&(t=n.alternate.memoizedState.cache),n=n.memoizedState.cache,n!==t&&(n.refCount++,t!=null&&so(t)));break;case 12:if(u&2048){Ti(t,n,a,r),t=n.stateNode;try{var f=n.memoizedProps,_=f.id,A=f.onPostCommit;typeof A=="function"&&A(_,n.alternate===null?"mount":"update",t.passiveEffectDuration,-0)}catch(B){Ft(n,n.return,B)}}else Ti(t,n,a,r);break;case 31:Ti(t,n,a,r);break;case 13:Ti(t,n,a,r);break;case 23:break;case 22:f=n.stateNode,_=n.alternate,n.memoizedState!==null?f._visibility&2?Ti(t,n,a,r):So(t,n):f._visibility&2?Ti(t,n,a,r):(f._visibility|=2,lr(t,n,a,r,(n.subtreeFlags&10256)!==0||!1)),u&2048&&Ef(_,n);break;case 24:Ti(t,n,a,r),u&2048&&Tf(n.alternate,n);break;default:Ti(t,n,a,r)}}function lr(t,n,a,r,u){for(u=u&&((n.subtreeFlags&10256)!==0||!1),n=n.child;n!==null;){var f=t,_=n,A=a,B=r,$=_.flags;switch(_.tag){case 0:case 11:case 15:lr(f,_,A,B,u),xo(8,_);break;case 23:break;case 22:var he=_.stateNode;_.memoizedState!==null?he._visibility&2?lr(f,_,A,B,u):So(f,_):(he._visibility|=2,lr(f,_,A,B,u)),u&&$&2048&&Ef(_.alternate,_);break;case 24:lr(f,_,A,B,u),u&&$&2048&&Tf(_.alternate,_);break;default:lr(f,_,A,B,u)}n=n.sibling}}function So(t,n){if(n.subtreeFlags&10256)for(n=n.child;n!==null;){var a=t,r=n,u=r.flags;switch(r.tag){case 22:So(a,r),u&2048&&Ef(r.alternate,r);break;case 24:So(a,r),u&2048&&Tf(r.alternate,r);break;default:So(a,r)}n=n.sibling}}var bo=8192;function cr(t,n,a){if(t.subtreeFlags&bo)for(t=t.child;t!==null;)Mg(t,n,a),t=t.sibling}function Mg(t,n,a){switch(t.tag){case 26:cr(t,n,a),t.flags&bo&&t.memoizedState!==null&&Cy(a,Ei,t.memoizedState,t.memoizedProps);break;case 5:cr(t,n,a);break;case 3:case 4:var r=Ei;Ei=ec(t.stateNode.containerInfo),cr(t,n,a),Ei=r;break;case 22:t.memoizedState===null&&(r=t.alternate,r!==null&&r.memoizedState!==null?(r=bo,bo=16777216,cr(t,n,a),bo=r):cr(t,n,a));break;default:cr(t,n,a)}}function Eg(t){var n=t.alternate;if(n!==null&&(t=n.child,t!==null)){n.child=null;do n=t.sibling,t.sibling=null,t=n;while(t!==null)}}function Mo(t){var n=t.deletions;if((t.flags&16)!==0){if(n!==null)for(var a=0;a<n.length;a++){var r=n[a];yn=r,Ag(r,t)}Eg(t)}if(t.subtreeFlags&10256)for(t=t.child;t!==null;)Tg(t),t=t.sibling}function Tg(t){switch(t.tag){case 0:case 11:case 15:Mo(t),t.flags&2048&&Fa(9,t,t.return);break;case 3:Mo(t);break;case 12:Mo(t);break;case 22:var n=t.stateNode;t.memoizedState!==null&&n._visibility&2&&(t.return===null||t.return.tag!==13)?(n._visibility&=-3,Gl(t)):Mo(t);break;default:Mo(t)}}function Gl(t){var n=t.deletions;if((t.flags&16)!==0){if(n!==null)for(var a=0;a<n.length;a++){var r=n[a];yn=r,Ag(r,t)}Eg(t)}for(t=t.child;t!==null;){switch(n=t,n.tag){case 0:case 11:case 15:Fa(8,n,n.return),Gl(n);break;case 22:a=n.stateNode,a._visibility&2&&(a._visibility&=-3,Gl(n));break;default:Gl(n)}t=t.sibling}}function Ag(t,n){for(;yn!==null;){var a=yn;switch(a.tag){case 0:case 11:case 15:Fa(8,a,n);break;case 23:case 22:if(a.memoizedState!==null&&a.memoizedState.cachePool!==null){var r=a.memoizedState.cachePool.pool;r!=null&&r.refCount++}break;case 24:so(a.memoizedState.cache)}if(r=a.child,r!==null)r.return=a,yn=r;else e:for(a=t;yn!==null;){r=yn;var u=r.sibling,f=r.return;if(gg(r),r===a){yn=null;break e}if(u!==null){u.return=f,yn=u;break e}yn=f}}}var Xx={getCacheForType:function(t){var n=Mn(cn),a=n.data.get(t);return a===void 0&&(a=t(),n.data.set(t,a)),a},cacheSignal:function(){return Mn(cn).controller.signal}},jx=typeof WeakMap=="function"?WeakMap:Map,Ut=0,kt=null,gt=null,yt=0,It=0,ei=null,za=!1,ur=!1,Af=!1,ua=0,tn=0,Ba=0,Es=0,Rf=0,ti=0,fr=0,Eo=null,kn=null,wf=!1,Vl=0,Rg=0,kl=1/0,Xl=null,Ha=null,gn=0,Ga=null,dr=null,fa=0,Cf=0,Df=null,wg=null,To=0,Nf=null;function ni(){return(Ut&2)!==0&&yt!==0?yt&-yt:P.T!==null?Ff():Ui()}function Cg(){if(ti===0)if((yt&536870912)===0||bt){var t=Ae;Ae<<=1,(Ae&3932160)===0&&(Ae=262144),ti=t}else ti=536870912;return t=Jn.current,t!==null&&(t.flags|=32),ti}function Xn(t,n,a){(t===kt&&(It===2||It===9)||t.cancelPendingCommit!==null)&&(hr(t,0),Va(t,yt,ti,!1)),Nn(t,a),((Ut&2)===0||t!==kt)&&(t===kt&&((Ut&2)===0&&(Es|=a),tn===4&&Va(t,yt,ti,!1)),Ii(t))}function Dg(t,n,a){if((Ut&6)!==0)throw Error(s(327));var r=!a&&(n&127)===0&&(n&t.expiredLanes)===0||Be(t,n),u=r?Yx(t,n):Lf(t,n,!0),f=r;do{if(u===0){ur&&!r&&Va(t,n,0,!1);break}else{if(a=t.current.alternate,f&&!Wx(a)){u=Lf(t,n,!1),f=!1;continue}if(u===2){if(f=n,t.errorRecoveryDisabledLanes&f)var _=0;else _=t.pendingLanes&-536870913,_=_!==0?_:_&536870912?536870912:0;if(_!==0){n=_;e:{var A=t;u=Eo;var B=A.current.memoizedState.isDehydrated;if(B&&(hr(A,_).flags|=256),_=Lf(A,_,!1),_!==2){if(Af&&!B){A.errorRecoveryDisabledLanes|=f,Es|=f,u=4;break e}f=kn,kn=u,f!==null&&(kn===null?kn=f:kn.push.apply(kn,f))}u=_}if(f=!1,u!==2)continue}}if(u===1){hr(t,0),Va(t,n,0,!0);break}e:{switch(r=t,f=u,f){case 0:case 1:throw Error(s(345));case 4:if((n&4194048)!==n)break;case 6:Va(r,n,ti,!za);break e;case 2:kn=null;break;case 3:case 5:break;default:throw Error(s(329))}if((n&62914560)===n&&(u=Vl+300-M(),10<u)){if(Va(r,n,ti,!za),fe(r,0,!0)!==0)break e;fa=n,r.timeoutHandle=o0(Ng.bind(null,r,a,kn,Xl,wf,n,ti,Es,fr,za,f,"Throttled",-0,0),u);break e}Ng(r,a,kn,Xl,wf,n,ti,Es,fr,za,f,null,-0,0)}}break}while(!0);Ii(t)}function Ng(t,n,a,r,u,f,_,A,B,$,he,_e,ae,oe){if(t.timeoutHandle=-1,_e=n.subtreeFlags,_e&8192||(_e&16785408)===16785408){_e={stylesheets:null,count:0,imgCount:0,imgBytes:0,suspenseyImages:[],waitingForImages:!0,waitingForViewTransition:!1,unsuspend:Ki},Mg(n,f,_e);var Ge=(f&62914560)===f?Vl-M():(f&4194048)===f?Rg-M():0;if(Ge=Dy(_e,Ge),Ge!==null){fa=f,t.cancelPendingCommit=Ge(Bg.bind(null,t,n,f,a,r,u,_,A,B,he,_e,null,ae,oe)),Va(t,f,_,!$);return}}Bg(t,n,f,a,r,u,_,A,B)}function Wx(t){for(var n=t;;){var a=n.tag;if((a===0||a===11||a===15)&&n.flags&16384&&(a=n.updateQueue,a!==null&&(a=a.stores,a!==null)))for(var r=0;r<a.length;r++){var u=a[r],f=u.getSnapshot;u=u.value;try{if(!Kn(f(),u))return!1}catch{return!1}}if(a=n.child,n.subtreeFlags&16384&&a!==null)a.return=n,n=a;else{if(n===t)break;for(;n.sibling===null;){if(n.return===null||n.return===t)return!0;n=n.return}n.sibling.return=n.return,n=n.sibling}}return!0}function Va(t,n,a,r){n&=~Rf,n&=~Es,t.suspendedLanes|=n,t.pingedLanes&=~n,r&&(t.warmLanes|=n),r=t.expirationTimes;for(var u=n;0<u;){var f=31-Le(u),_=1<<f;r[f]=-1,u&=~_}a!==0&&Wr(t,a,n)}function jl(){return(Ut&6)===0?(Ao(0),!1):!0}function Uf(){if(gt!==null){if(It===0)var t=gt.return;else t=gt,ea=ms=null,qu(t),ir=null,oo=0,t=gt;for(;t!==null;)lg(t.alternate,t),t=t.return;gt=null}}function hr(t,n){var a=t.timeoutHandle;a!==-1&&(t.timeoutHandle=-1,dy(a)),a=t.cancelPendingCommit,a!==null&&(t.cancelPendingCommit=null,a()),fa=0,Uf(),kt=t,gt=a=Ji(t.current,null),yt=n,It=0,ei=null,za=!1,ur=Be(t,n),Af=!1,fr=ti=Rf=Es=Ba=tn=0,kn=Eo=null,wf=!1,(n&8)!==0&&(n|=n&32);var r=t.entangledLanes;if(r!==0)for(t=t.entanglements,r&=n;0<r;){var u=31-Le(r),f=1<<u;n|=t[u],r&=~f}return ua=n,dl(),a}function Ug(t,n){lt=null,P.H=go,n===nr||n===yl?(n=Yp(),It=3):n===Pu?(n=Yp(),It=4):It=n===uf?8:n!==null&&typeof n=="object"&&typeof n.then=="function"?6:1,ei=n,gt===null&&(tn=1,Ol(t,li(n,t.current)))}function Lg(){var t=Jn.current;return t===null?!0:(yt&4194048)===yt?di===null:(yt&62914560)===yt||(yt&536870912)!==0?t===di:!1}function Og(){var t=P.H;return P.H=go,t===null?go:t}function Pg(){var t=P.A;return P.A=Xx,t}function Wl(){tn=4,za||(yt&4194048)!==yt&&Jn.current!==null||(ur=!0),(Ba&134217727)===0&&(Es&134217727)===0||kt===null||Va(kt,yt,ti,!1)}function Lf(t,n,a){var r=Ut;Ut|=2;var u=Og(),f=Pg();(kt!==t||yt!==n)&&(Xl=null,hr(t,n)),n=!1;var _=tn;e:do try{if(It!==0&&gt!==null){var A=gt,B=ei;switch(It){case 8:Uf(),_=6;break e;case 3:case 2:case 9:case 6:Jn.current===null&&(n=!0);var $=It;if(It=0,ei=null,pr(t,A,B,$),a&&ur){_=0;break e}break;default:$=It,It=0,ei=null,pr(t,A,B,$)}}qx(),_=tn;break}catch(he){Ug(t,he)}while(!0);return n&&t.shellSuspendCounter++,ea=ms=null,Ut=r,P.H=u,P.A=f,gt===null&&(kt=null,yt=0,dl()),_}function qx(){for(;gt!==null;)Ig(gt)}function Yx(t,n){var a=Ut;Ut|=2;var r=Og(),u=Pg();kt!==t||yt!==n?(Xl=null,kl=M()+500,hr(t,n)):ur=Be(t,n);e:do try{if(It!==0&&gt!==null){n=gt;var f=ei;t:switch(It){case 1:It=0,ei=null,pr(t,n,f,1);break;case 2:case 9:if(Wp(f)){It=0,ei=null,Fg(n);break}n=function(){It!==2&&It!==9||kt!==t||(It=7),Ii(t)},f.then(n,n);break e;case 3:It=7;break e;case 4:It=5;break e;case 7:Wp(f)?(It=0,ei=null,Fg(n)):(It=0,ei=null,pr(t,n,f,7));break;case 5:var _=null;switch(gt.tag){case 26:_=gt.memoizedState;case 5:case 27:var A=gt;if(_?b0(_):A.stateNode.complete){It=0,ei=null;var B=A.sibling;if(B!==null)gt=B;else{var $=A.return;$!==null?(gt=$,ql($)):gt=null}break t}}It=0,ei=null,pr(t,n,f,5);break;case 6:It=0,ei=null,pr(t,n,f,6);break;case 8:Uf(),tn=6;break e;default:throw Error(s(462))}}Zx();break}catch(he){Ug(t,he)}while(!0);return ea=ms=null,P.H=r,P.A=u,Ut=a,gt!==null?0:(kt=null,yt=0,dl(),tn)}function Zx(){for(;gt!==null&&!We();)Ig(gt)}function Ig(t){var n=rg(t.alternate,t,ua);t.memoizedProps=t.pendingProps,n===null?ql(t):gt=n}function Fg(t){var n=t,a=n.alternate;switch(n.tag){case 15:case 0:n=eg(a,n,n.pendingProps,n.type,void 0,yt);break;case 11:n=eg(a,n,n.pendingProps,n.type.render,n.ref,yt);break;case 5:qu(n);default:lg(a,n),n=gt=Pp(n,ua),n=rg(a,n,ua)}t.memoizedProps=t.pendingProps,n===null?ql(t):gt=n}function pr(t,n,a,r){ea=ms=null,qu(n),ir=null,oo=0;var u=n.return;try{if(Fx(t,u,n,a,yt)){tn=1,Ol(t,li(a,t.current)),gt=null;return}}catch(f){if(u!==null)throw gt=u,f;tn=1,Ol(t,li(a,t.current)),gt=null;return}n.flags&32768?(bt||r===1?t=!0:ur||(yt&536870912)!==0?t=!1:(za=t=!0,(r===2||r===9||r===3||r===6)&&(r=Jn.current,r!==null&&r.tag===13&&(r.flags|=16384))),zg(n,t)):ql(n)}function ql(t){var n=t;do{if((n.flags&32768)!==0){zg(n,za);return}t=n.return;var a=Hx(n.alternate,n,ua);if(a!==null){gt=a;return}if(n=n.sibling,n!==null){gt=n;return}gt=n=t}while(n!==null);tn===0&&(tn=5)}function zg(t,n){do{var a=Gx(t.alternate,t);if(a!==null){a.flags&=32767,gt=a;return}if(a=t.return,a!==null&&(a.flags|=32768,a.subtreeFlags=0,a.deletions=null),!n&&(t=t.sibling,t!==null)){gt=t;return}gt=t=a}while(t!==null);tn=6,gt=null}function Bg(t,n,a,r,u,f,_,A,B){t.cancelPendingCommit=null;do Yl();while(gn!==0);if((Ut&6)!==0)throw Error(s(327));if(n!==null){if(n===t.current)throw Error(s(177));if(f=n.lanes|n.childLanes,f|=yu,xi(t,a,f,_,A,B),t===kt&&(gt=kt=null,yt=0),dr=n,Ga=t,fa=a,Cf=f,Df=u,wg=r,(n.subtreeFlags&10256)!==0||(n.flags&10256)!==0?(t.callbackNode=null,t.callbackPriority=0,$x(de,function(){return Xg(),null})):(t.callbackNode=null,t.callbackPriority=0),r=(n.flags&13878)!==0,(n.subtreeFlags&13878)!==0||r){r=P.T,P.T=null,u=z.p,z.p=2,_=Ut,Ut|=4;try{Vx(t,n,a)}finally{Ut=_,z.p=u,P.T=r}}gn=1,Hg(),Gg(),Vg()}}function Hg(){if(gn===1){gn=0;var t=Ga,n=dr,a=(n.flags&13878)!==0;if((n.subtreeFlags&13878)!==0||a){a=P.T,P.T=null;var r=z.p;z.p=2;var u=Ut;Ut|=4;try{yg(n,t);var f=jf,_=Ap(t.containerInfo),A=f.focusedElem,B=f.selectionRange;if(_!==A&&A&&A.ownerDocument&&Tp(A.ownerDocument.documentElement,A)){if(B!==null&&mu(A)){var $=B.start,he=B.end;if(he===void 0&&(he=$),"selectionStart"in A)A.selectionStart=$,A.selectionEnd=Math.min(he,A.value.length);else{var _e=A.ownerDocument||document,ae=_e&&_e.defaultView||window;if(ae.getSelection){var oe=ae.getSelection(),Ge=A.textContent.length,Je=Math.min(B.start,Ge),Gt=B.end===void 0?Je:Math.min(B.end,Ge);!oe.extend&&Je>Gt&&(_=Gt,Gt=Je,Je=_);var Z=Ep(A,Je),X=Ep(A,Gt);if(Z&&X&&(oe.rangeCount!==1||oe.anchorNode!==Z.node||oe.anchorOffset!==Z.offset||oe.focusNode!==X.node||oe.focusOffset!==X.offset)){var J=_e.createRange();J.setStart(Z.node,Z.offset),oe.removeAllRanges(),Je>Gt?(oe.addRange(J),oe.extend(X.node,X.offset)):(J.setEnd(X.node,X.offset),oe.addRange(J))}}}}for(_e=[],oe=A;oe=oe.parentNode;)oe.nodeType===1&&_e.push({element:oe,left:oe.scrollLeft,top:oe.scrollTop});for(typeof A.focus=="function"&&A.focus(),A=0;A<_e.length;A++){var ge=_e[A];ge.element.scrollLeft=ge.left,ge.element.scrollTop=ge.top}}rc=!!Xf,jf=Xf=null}finally{Ut=u,z.p=r,P.T=a}}t.current=n,gn=2}}function Gg(){if(gn===2){gn=0;var t=Ga,n=dr,a=(n.flags&8772)!==0;if((n.subtreeFlags&8772)!==0||a){a=P.T,P.T=null;var r=z.p;z.p=2;var u=Ut;Ut|=4;try{mg(t,n.alternate,n)}finally{Ut=u,z.p=r,P.T=a}}gn=3}}function Vg(){if(gn===4||gn===3){gn=0,L();var t=Ga,n=dr,a=fa,r=wg;(n.subtreeFlags&10256)!==0||(n.flags&10256)!==0?gn=5:(gn=0,dr=Ga=null,kg(t,t.pendingLanes));var u=t.pendingLanes;if(u===0&&(Ha=null),Hs(a),n=n.stateNode,Se&&typeof Se.onCommitFiberRoot=="function")try{Se.onCommitFiberRoot(Me,n,void 0,(n.current.flags&128)===128)}catch{}if(r!==null){n=P.T,u=z.p,z.p=2,P.T=null;try{for(var f=t.onRecoverableError,_=0;_<r.length;_++){var A=r[_];f(A.value,{componentStack:A.stack})}}finally{P.T=n,z.p=u}}(fa&3)!==0&&Yl(),Ii(t),u=t.pendingLanes,(a&261930)!==0&&(u&42)!==0?t===Nf?To++:(To=0,Nf=t):To=0,Ao(0)}}function kg(t,n){(t.pooledCacheLanes&=n)===0&&(n=t.pooledCache,n!=null&&(t.pooledCache=null,so(n)))}function Yl(){return Hg(),Gg(),Vg(),Xg()}function Xg(){if(gn!==5)return!1;var t=Ga,n=Cf;Cf=0;var a=Hs(fa),r=P.T,u=z.p;try{z.p=32>a?32:a,P.T=null,a=Df,Df=null;var f=Ga,_=fa;if(gn=0,dr=Ga=null,fa=0,(Ut&6)!==0)throw Error(s(331));var A=Ut;if(Ut|=4,Tg(f.current),bg(f,f.current,_,a),Ut=A,Ao(0,!1),Se&&typeof Se.onPostCommitFiberRoot=="function")try{Se.onPostCommitFiberRoot(Me,f)}catch{}return!0}finally{z.p=u,P.T=r,kg(t,n)}}function jg(t,n,a){n=li(a,n),n=cf(t.stateNode,n,2),t=Oa(t,n,2),t!==null&&(Nn(t,2),Ii(t))}function Ft(t,n,a){if(t.tag===3)jg(t,t,a);else for(;n!==null;){if(n.tag===3){jg(n,t,a);break}else if(n.tag===1){var r=n.stateNode;if(typeof n.type.getDerivedStateFromError=="function"||typeof r.componentDidCatch=="function"&&(Ha===null||!Ha.has(r))){t=li(a,t),a=Wm(2),r=Oa(n,a,2),r!==null&&(qm(a,r,n,t),Nn(r,2),Ii(r));break}}n=n.return}}function Of(t,n,a){var r=t.pingCache;if(r===null){r=t.pingCache=new jx;var u=new Set;r.set(n,u)}else u=r.get(n),u===void 0&&(u=new Set,r.set(n,u));u.has(a)||(Af=!0,u.add(a),t=Kx.bind(null,t,n,a),n.then(t,t))}function Kx(t,n,a){var r=t.pingCache;r!==null&&r.delete(n),t.pingedLanes|=t.suspendedLanes&a,t.warmLanes&=~a,kt===t&&(yt&a)===a&&(tn===4||tn===3&&(yt&62914560)===yt&&300>M()-Vl?(Ut&2)===0&&hr(t,0):Rf|=a,fr===yt&&(fr=0)),Ii(t)}function Wg(t,n){n===0&&(n=Pt()),t=ds(t,n),t!==null&&(Nn(t,n),Ii(t))}function Qx(t){var n=t.memoizedState,a=0;n!==null&&(a=n.retryLane),Wg(t,a)}function Jx(t,n){var a=0;switch(t.tag){case 31:case 13:var r=t.stateNode,u=t.memoizedState;u!==null&&(a=u.retryLane);break;case 19:r=t.stateNode;break;case 22:r=t.stateNode._retryCache;break;default:throw Error(s(314))}r!==null&&r.delete(n),Wg(t,a)}function $x(t,n){return Mt(t,n)}var Zl=null,mr=null,Pf=!1,Kl=!1,If=!1,ka=0;function Ii(t){t!==mr&&t.next===null&&(mr===null?Zl=mr=t:mr=mr.next=t),Kl=!0,Pf||(Pf=!0,ty())}function Ao(t,n){if(!If&&Kl){If=!0;do for(var a=!1,r=Zl;r!==null;){if(t!==0){var u=r.pendingLanes;if(u===0)var f=0;else{var _=r.suspendedLanes,A=r.pingedLanes;f=(1<<31-Le(42|t)+1)-1,f&=u&~(_&~A),f=f&201326741?f&201326741|1:f?f|2:0}f!==0&&(a=!0,Kg(r,f))}else f=yt,f=fe(r,r===kt?f:0,r.cancelPendingCommit!==null||r.timeoutHandle!==-1),(f&3)===0||Be(r,f)||(a=!0,Kg(r,f));r=r.next}while(a);If=!1}}function ey(){qg()}function qg(){Kl=Pf=!1;var t=0;ka!==0&&fy()&&(t=ka);for(var n=M(),a=null,r=Zl;r!==null;){var u=r.next,f=Yg(r,n);f===0?(r.next=null,a===null?Zl=u:a.next=u,u===null&&(mr=a)):(a=r,(t!==0||(f&3)!==0)&&(Kl=!0)),r=u}gn!==0&&gn!==5||Ao(t),ka!==0&&(ka=0)}function Yg(t,n){for(var a=t.suspendedLanes,r=t.pingedLanes,u=t.expirationTimes,f=t.pendingLanes&-62914561;0<f;){var _=31-Le(f),A=1<<_,B=u[_];B===-1?((A&a)===0||(A&r)!==0)&&(u[_]=nt(A,n)):B<=n&&(t.expiredLanes|=A),f&=~A}if(n=kt,a=yt,a=fe(t,t===n?a:0,t.cancelPendingCommit!==null||t.timeoutHandle!==-1),r=t.callbackNode,a===0||t===n&&(It===2||It===9)||t.cancelPendingCommit!==null)return r!==null&&r!==null&&Lt(r),t.callbackNode=null,t.callbackPriority=0;if((a&3)===0||Be(t,a)){if(n=a&-a,n===t.callbackPriority)return n;switch(r!==null&&Lt(r),Hs(a)){case 2:case 8:a=ye;break;case 32:a=de;break;case 268435456:a=Ce;break;default:a=de}return r=Zg.bind(null,t),a=Mt(a,r),t.callbackPriority=n,t.callbackNode=a,n}return r!==null&&r!==null&&Lt(r),t.callbackPriority=2,t.callbackNode=null,2}function Zg(t,n){if(gn!==0&&gn!==5)return t.callbackNode=null,t.callbackPriority=0,null;var a=t.callbackNode;if(Yl()&&t.callbackNode!==a)return null;var r=yt;return r=fe(t,t===kt?r:0,t.cancelPendingCommit!==null||t.timeoutHandle!==-1),r===0?null:(Dg(t,r,n),Yg(t,M()),t.callbackNode!=null&&t.callbackNode===a?Zg.bind(null,t):null)}function Kg(t,n){if(Yl())return null;Dg(t,n,!0)}function ty(){hy(function(){(Ut&6)!==0?Mt(me,ey):qg()})}function Ff(){if(ka===0){var t=er;t===0&&(t=we,we<<=1,(we&261888)===0&&(we=256)),ka=t}return ka}function Qg(t){return t==null||typeof t=="symbol"||typeof t=="boolean"?null:typeof t=="function"?t:al(""+t)}function Jg(t,n){var a=n.ownerDocument.createElement("input");return a.name=n.name,a.value=n.value,t.id&&a.setAttribute("form",t.id),n.parentNode.insertBefore(a,n),t=new FormData(t),a.parentNode.removeChild(a),t}function ny(t,n,a,r,u){if(n==="submit"&&a&&a.stateNode===u){var f=Qg((u[pn]||null).action),_=r.submitter;_&&(n=(n=_[pn]||null)?Qg(n.formAction):_.getAttribute("formAction"),n!==null&&(f=n,_=null));var A=new ll("action","action",null,r,u);t.push({event:A,listeners:[{instance:null,listener:function(){if(r.defaultPrevented){if(ka!==0){var B=_?Jg(u,_):new FormData(u);nf(a,{pending:!0,data:B,method:u.method,action:f},null,B)}}else typeof f=="function"&&(A.preventDefault(),B=_?Jg(u,_):new FormData(u),nf(a,{pending:!0,data:B,method:u.method,action:f},f,B))},currentTarget:u}]})}}for(var zf=0;zf<xu.length;zf++){var Bf=xu[zf],iy=Bf.toLowerCase(),ay=Bf[0].toUpperCase()+Bf.slice(1);Mi(iy,"on"+ay)}Mi(Cp,"onAnimationEnd"),Mi(Dp,"onAnimationIteration"),Mi(Np,"onAnimationStart"),Mi("dblclick","onDoubleClick"),Mi("focusin","onFocus"),Mi("focusout","onBlur"),Mi(yx,"onTransitionRun"),Mi(Sx,"onTransitionStart"),Mi(bx,"onTransitionCancel"),Mi(Up,"onTransitionEnd"),De("onMouseEnter",["mouseout","mouseover"]),De("onMouseLeave",["mouseout","mouseover"]),De("onPointerEnter",["pointerout","pointerover"]),De("onPointerLeave",["pointerout","pointerover"]),Q("onChange","change click focusin focusout input keydown keyup selectionchange".split(" ")),Q("onSelect","focusout contextmenu dragend focusin keydown keyup mousedown mouseup selectionchange".split(" ")),Q("onBeforeInput",["compositionend","keypress","textInput","paste"]),Q("onCompositionEnd","compositionend focusout keydown keypress keyup mousedown".split(" ")),Q("onCompositionStart","compositionstart focusout keydown keypress keyup mousedown".split(" ")),Q("onCompositionUpdate","compositionupdate focusout keydown keypress keyup mousedown".split(" "));var Ro="abort canplay canplaythrough durationchange emptied encrypted ended error loadeddata loadedmetadata loadstart pause play playing progress ratechange resize seeked seeking stalled suspend timeupdate volumechange waiting".split(" "),sy=new Set("beforetoggle cancel close invalid load scroll scrollend toggle".split(" ").concat(Ro));function $g(t,n){n=(n&4)!==0;for(var a=0;a<t.length;a++){var r=t[a],u=r.event;r=r.listeners;e:{var f=void 0;if(n)for(var _=r.length-1;0<=_;_--){var A=r[_],B=A.instance,$=A.currentTarget;if(A=A.listener,B!==f&&u.isPropagationStopped())break e;f=A,u.currentTarget=$;try{f(u)}catch(he){fl(he)}u.currentTarget=null,f=B}else for(_=0;_<r.length;_++){if(A=r[_],B=A.instance,$=A.currentTarget,A=A.listener,B!==f&&u.isPropagationStopped())break e;f=A,u.currentTarget=$;try{f(u)}catch(he){fl(he)}u.currentTarget=null,f=B}}}}function _t(t,n){var a=n[Ea];a===void 0&&(a=n[Ea]=new Set);var r=t+"__bubble";a.has(r)||(e0(n,t,2,!1),a.add(r))}function Hf(t,n,a){var r=0;n&&(r|=4),e0(a,t,r,n)}var Ql="_reactListening"+Math.random().toString(36).slice(2);function Gf(t){if(!t[Ql]){t[Ql]=!0,re.forEach(function(a){a!=="selectionchange"&&(sy.has(a)||Hf(a,!1,t),Hf(a,!0,t))});var n=t.nodeType===9?t:t.ownerDocument;n===null||n[Ql]||(n[Ql]=!0,Hf("selectionchange",!1,n))}}function e0(t,n,a,r){switch(C0(n)){case 2:var u=Ly;break;case 8:u=Oy;break;default:u=nd}a=u.bind(null,n,a,t),u=void 0,!ru||n!=="touchstart"&&n!=="touchmove"&&n!=="wheel"||(u=!0),r?u!==void 0?t.addEventListener(n,a,{capture:!0,passive:u}):t.addEventListener(n,a,!0):u!==void 0?t.addEventListener(n,a,{passive:u}):t.addEventListener(n,a,!1)}function Vf(t,n,a,r,u){var f=r;if((n&1)===0&&(n&2)===0&&r!==null)e:for(;;){if(r===null)return;var _=r.tag;if(_===3||_===4){var A=r.stateNode.containerInfo;if(A===u)break;if(_===4)for(_=r.return;_!==null;){var B=_.tag;if((B===3||B===4)&&_.stateNode.containerInfo===u)return;_=_.return}for(;A!==null;){if(_=Ta(A),_===null)return;if(B=_.tag,B===5||B===6||B===26||B===27){r=f=_;continue e}A=A.parentNode}}r=r.return}sp(function(){var $=f,he=au(a),_e=[];e:{var ae=Lp.get(t);if(ae!==void 0){var oe=ll,Ge=t;switch(t){case"keypress":if(rl(a)===0)break e;case"keydown":case"keyup":oe=Jv;break;case"focusin":Ge="focus",oe=uu;break;case"focusout":Ge="blur",oe=uu;break;case"beforeblur":case"afterblur":oe=uu;break;case"click":if(a.button===2)break e;case"auxclick":case"dblclick":case"mousedown":case"mousemove":case"mouseup":case"mouseout":case"mouseover":case"contextmenu":oe=lp;break;case"drag":case"dragend":case"dragenter":case"dragexit":case"dragleave":case"dragover":case"dragstart":case"drop":oe=Hv;break;case"touchcancel":case"touchend":case"touchmove":case"touchstart":oe=tx;break;case Cp:case Dp:case Np:oe=kv;break;case Up:oe=ix;break;case"scroll":case"scrollend":oe=zv;break;case"wheel":oe=sx;break;case"copy":case"cut":case"paste":oe=jv;break;case"gotpointercapture":case"lostpointercapture":case"pointercancel":case"pointerdown":case"pointermove":case"pointerout":case"pointerover":case"pointerup":oe=up;break;case"toggle":case"beforetoggle":oe=ox}var Je=(n&4)!==0,Gt=!Je&&(t==="scroll"||t==="scrollend"),Z=Je?ae!==null?ae+"Capture":null:ae;Je=[];for(var X=$,J;X!==null;){var ge=X;if(J=ge.stateNode,ge=ge.tag,ge!==5&&ge!==26&&ge!==27||J===null||Z===null||(ge=Yr(X,Z),ge!=null&&Je.push(wo(X,ge,J))),Gt)break;X=X.return}0<Je.length&&(ae=new oe(ae,Ge,null,a,he),_e.push({event:ae,listeners:Je}))}}if((n&7)===0){e:{if(ae=t==="mouseover"||t==="pointerover",oe=t==="mouseout"||t==="pointerout",ae&&a!==iu&&(Ge=a.relatedTarget||a.fromElement)&&(Ta(Ge)||Ge[Yi]))break e;if((oe||ae)&&(ae=he.window===he?he:(ae=he.ownerDocument)?ae.defaultView||ae.parentWindow:window,oe?(Ge=a.relatedTarget||a.toElement,oe=$,Ge=Ge?Ta(Ge):null,Ge!==null&&(Gt=c(Ge),Je=Ge.tag,Ge!==Gt||Je!==5&&Je!==27&&Je!==6)&&(Ge=null)):(oe=null,Ge=$),oe!==Ge)){if(Je=lp,ge="onMouseLeave",Z="onMouseEnter",X="mouse",(t==="pointerout"||t==="pointerover")&&(Je=up,ge="onPointerLeave",Z="onPointerEnter",X="pointer"),Gt=oe==null?ae:ls(oe),J=Ge==null?ae:ls(Ge),ae=new Je(ge,X+"leave",oe,a,he),ae.target=Gt,ae.relatedTarget=J,ge=null,Ta(he)===$&&(Je=new Je(Z,X+"enter",Ge,a,he),Je.target=J,Je.relatedTarget=Gt,ge=Je),Gt=ge,oe&&Ge)t:{for(Je=ry,Z=oe,X=Ge,J=0,ge=Z;ge;ge=Je(ge))J++;ge=0;for(var Qe=X;Qe;Qe=Je(Qe))ge++;for(;0<J-ge;)Z=Je(Z),J--;for(;0<ge-J;)X=Je(X),ge--;for(;J--;){if(Z===X||X!==null&&Z===X.alternate){Je=Z;break t}Z=Je(Z),X=Je(X)}Je=null}else Je=null;oe!==null&&t0(_e,ae,oe,Je,!1),Ge!==null&&Gt!==null&&t0(_e,Gt,Ge,Je,!0)}}e:{if(ae=$?ls($):window,oe=ae.nodeName&&ae.nodeName.toLowerCase(),oe==="select"||oe==="input"&&ae.type==="file")var wt=vp;else if(gp(ae))if(xp)wt=_x;else{wt=mx;var qe=px}else oe=ae.nodeName,!oe||oe.toLowerCase()!=="input"||ae.type!=="checkbox"&&ae.type!=="radio"?$&&Vs($.elementType)&&(wt=vp):wt=gx;if(wt&&(wt=wt(t,$))){_p(_e,wt,a,he);break e}qe&&qe(t,ae,$),t==="focusout"&&$&&ae.type==="number"&&$.memoizedProps.value!=null&&Si(ae,"number",ae.value)}switch(qe=$?ls($):window,t){case"focusin":(gp(qe)||qe.contentEditable==="true")&&(Ws=qe,gu=$,no=null);break;case"focusout":no=gu=Ws=null;break;case"mousedown":_u=!0;break;case"contextmenu":case"mouseup":case"dragend":_u=!1,Rp(_e,a,he);break;case"selectionchange":if(xx)break;case"keydown":case"keyup":Rp(_e,a,he)}var dt;if(du)e:{switch(t){case"compositionstart":var St="onCompositionStart";break e;case"compositionend":St="onCompositionEnd";break e;case"compositionupdate":St="onCompositionUpdate";break e}St=void 0}else js?pp(t,a)&&(St="onCompositionEnd"):t==="keydown"&&a.keyCode===229&&(St="onCompositionStart");St&&(fp&&a.locale!=="ko"&&(js||St!=="onCompositionStart"?St==="onCompositionEnd"&&js&&(dt=rp()):(Ra=he,ou="value"in Ra?Ra.value:Ra.textContent,js=!0)),qe=Jl($,St),0<qe.length&&(St=new cp(St,t,null,a,he),_e.push({event:St,listeners:qe}),dt?St.data=dt:(dt=mp(a),dt!==null&&(St.data=dt)))),(dt=cx?ux(t,a):fx(t,a))&&(St=Jl($,"onBeforeInput"),0<St.length&&(qe=new cp("onBeforeInput","beforeinput",null,a,he),_e.push({event:qe,listeners:St}),qe.data=dt)),ny(_e,t,$,a,he)}$g(_e,n)})}function wo(t,n,a){return{instance:t,listener:n,currentTarget:a}}function Jl(t,n){for(var a=n+"Capture",r=[];t!==null;){var u=t,f=u.stateNode;if(u=u.tag,u!==5&&u!==26&&u!==27||f===null||(u=Yr(t,a),u!=null&&r.unshift(wo(t,u,f)),u=Yr(t,n),u!=null&&r.push(wo(t,u,f))),t.tag===3)return r;t=t.return}return[]}function ry(t){if(t===null)return null;do t=t.return;while(t&&t.tag!==5&&t.tag!==27);return t||null}function t0(t,n,a,r,u){for(var f=n._reactName,_=[];a!==null&&a!==r;){var A=a,B=A.alternate,$=A.stateNode;if(A=A.tag,B!==null&&B===r)break;A!==5&&A!==26&&A!==27||$===null||(B=$,u?($=Yr(a,f),$!=null&&_.unshift(wo(a,$,B))):u||($=Yr(a,f),$!=null&&_.push(wo(a,$,B)))),a=a.return}_.length!==0&&t.push({event:n,listeners:_})}var oy=/\r\n?/g,ly=/\u0000|\uFFFD/g;function n0(t){return(typeof t=="string"?t:""+t).replace(oy,`
`).replace(ly,"")}function i0(t,n){return n=n0(n),n0(t)===n}function Ht(t,n,a,r,u,f){switch(a){case"children":typeof r=="string"?n==="body"||n==="textarea"&&r===""||On(t,r):(typeof r=="number"||typeof r=="bigint")&&n!=="body"&&On(t,""+r);break;case"className":st(t,"class",r);break;case"tabIndex":st(t,"tabindex",r);break;case"dir":case"role":case"viewBox":case"width":case"height":st(t,a,r);break;case"style":Zi(t,r,f);break;case"data":if(n!=="object"){st(t,"data",r);break}case"src":case"href":if(r===""&&(n!=="a"||a!=="href")){t.removeAttribute(a);break}if(r==null||typeof r=="function"||typeof r=="symbol"||typeof r=="boolean"){t.removeAttribute(a);break}r=al(""+r),t.setAttribute(a,r);break;case"action":case"formAction":if(typeof r=="function"){t.setAttribute(a,"javascript:throw new Error('A React form was unexpectedly submitted. If you called form.submit() manually, consider using form.requestSubmit() instead. If you\\'re trying to use event.stopPropagation() in a submit event handler, consider also calling event.preventDefault().')");break}else typeof f=="function"&&(a==="formAction"?(n!=="input"&&Ht(t,n,"name",u.name,u,null),Ht(t,n,"formEncType",u.formEncType,u,null),Ht(t,n,"formMethod",u.formMethod,u,null),Ht(t,n,"formTarget",u.formTarget,u,null)):(Ht(t,n,"encType",u.encType,u,null),Ht(t,n,"method",u.method,u,null),Ht(t,n,"target",u.target,u,null)));if(r==null||typeof r=="symbol"||typeof r=="boolean"){t.removeAttribute(a);break}r=al(""+r),t.setAttribute(a,r);break;case"onClick":r!=null&&(t.onclick=Ki);break;case"onScroll":r!=null&&_t("scroll",t);break;case"onScrollEnd":r!=null&&_t("scrollend",t);break;case"dangerouslySetInnerHTML":if(r!=null){if(typeof r!="object"||!("__html"in r))throw Error(s(61));if(a=r.__html,a!=null){if(u.children!=null)throw Error(s(60));t.innerHTML=a}}break;case"multiple":t.multiple=r&&typeof r!="function"&&typeof r!="symbol";break;case"muted":t.muted=r&&typeof r!="function"&&typeof r!="symbol";break;case"suppressContentEditableWarning":case"suppressHydrationWarning":case"defaultValue":case"defaultChecked":case"innerHTML":case"ref":break;case"autoFocus":break;case"xlinkHref":if(r==null||typeof r=="function"||typeof r=="boolean"||typeof r=="symbol"){t.removeAttribute("xlink:href");break}a=al(""+r),t.setAttributeNS("http://www.w3.org/1999/xlink","xlink:href",a);break;case"contentEditable":case"spellCheck":case"draggable":case"value":case"autoReverse":case"externalResourcesRequired":case"focusable":case"preserveAlpha":r!=null&&typeof r!="function"&&typeof r!="symbol"?t.setAttribute(a,""+r):t.removeAttribute(a);break;case"inert":case"allowFullScreen":case"async":case"autoPlay":case"controls":case"default":case"defer":case"disabled":case"disablePictureInPicture":case"disableRemotePlayback":case"formNoValidate":case"hidden":case"loop":case"noModule":case"noValidate":case"open":case"playsInline":case"readOnly":case"required":case"reversed":case"scoped":case"seamless":case"itemScope":r&&typeof r!="function"&&typeof r!="symbol"?t.setAttribute(a,""):t.removeAttribute(a);break;case"capture":case"download":r===!0?t.setAttribute(a,""):r!==!1&&r!=null&&typeof r!="function"&&typeof r!="symbol"?t.setAttribute(a,r):t.removeAttribute(a);break;case"cols":case"rows":case"size":case"span":r!=null&&typeof r!="function"&&typeof r!="symbol"&&!isNaN(r)&&1<=r?t.setAttribute(a,r):t.removeAttribute(a);break;case"rowSpan":case"start":r==null||typeof r=="function"||typeof r=="symbol"||isNaN(r)?t.removeAttribute(a):t.setAttribute(a,r);break;case"popover":_t("beforetoggle",t),_t("toggle",t),tt(t,"popover",r);break;case"xlinkActuate":He(t,"http://www.w3.org/1999/xlink","xlink:actuate",r);break;case"xlinkArcrole":He(t,"http://www.w3.org/1999/xlink","xlink:arcrole",r);break;case"xlinkRole":He(t,"http://www.w3.org/1999/xlink","xlink:role",r);break;case"xlinkShow":He(t,"http://www.w3.org/1999/xlink","xlink:show",r);break;case"xlinkTitle":He(t,"http://www.w3.org/1999/xlink","xlink:title",r);break;case"xlinkType":He(t,"http://www.w3.org/1999/xlink","xlink:type",r);break;case"xmlBase":He(t,"http://www.w3.org/XML/1998/namespace","xml:base",r);break;case"xmlLang":He(t,"http://www.w3.org/XML/1998/namespace","xml:lang",r);break;case"xmlSpace":He(t,"http://www.w3.org/XML/1998/namespace","xml:space",r);break;case"is":tt(t,"is",r);break;case"innerText":case"textContent":break;default:(!(2<a.length)||a[0]!=="o"&&a[0]!=="O"||a[1]!=="n"&&a[1]!=="N")&&(a=Iv.get(a)||a,tt(t,a,r))}}function kf(t,n,a,r,u,f){switch(a){case"style":Zi(t,r,f);break;case"dangerouslySetInnerHTML":if(r!=null){if(typeof r!="object"||!("__html"in r))throw Error(s(61));if(a=r.__html,a!=null){if(u.children!=null)throw Error(s(60));t.innerHTML=a}}break;case"children":typeof r=="string"?On(t,r):(typeof r=="number"||typeof r=="bigint")&&On(t,""+r);break;case"onScroll":r!=null&&_t("scroll",t);break;case"onScrollEnd":r!=null&&_t("scrollend",t);break;case"onClick":r!=null&&(t.onclick=Ki);break;case"suppressContentEditableWarning":case"suppressHydrationWarning":case"innerHTML":case"ref":break;case"innerText":case"textContent":break;default:if(!ne.hasOwnProperty(a))e:{if(a[0]==="o"&&a[1]==="n"&&(u=a.endsWith("Capture"),n=a.slice(2,u?a.length-7:void 0),f=t[pn]||null,f=f!=null?f[a]:null,typeof f=="function"&&t.removeEventListener(n,f,u),typeof r=="function")){typeof f!="function"&&f!==null&&(a in t?t[a]=null:t.hasAttribute(a)&&t.removeAttribute(a)),t.addEventListener(n,r,u);break e}a in t?t[a]=r:r===!0?t.setAttribute(a,""):tt(t,a,r)}}}function Tn(t,n,a){switch(n){case"div":case"span":case"svg":case"path":case"a":case"g":case"p":case"li":break;case"img":_t("error",t),_t("load",t);var r=!1,u=!1,f;for(f in a)if(a.hasOwnProperty(f)){var _=a[f];if(_!=null)switch(f){case"src":r=!0;break;case"srcSet":u=!0;break;case"children":case"dangerouslySetInnerHTML":throw Error(s(137,n));default:Ht(t,n,f,_,a,null)}}u&&Ht(t,n,"srcSet",a.srcSet,a,null),r&&Ht(t,n,"src",a.src,a,null);return;case"input":_t("invalid",t);var A=f=_=u=null,B=null,$=null;for(r in a)if(a.hasOwnProperty(r)){var he=a[r];if(he!=null)switch(r){case"name":u=he;break;case"type":_=he;break;case"checked":B=he;break;case"defaultChecked":$=he;break;case"value":f=he;break;case"defaultValue":A=he;break;case"children":case"dangerouslySetInnerHTML":if(he!=null)throw Error(s(137,n));break;default:Ht(t,n,r,he,a,null)}}Yn(t,f,A,B,$,_,u,!1);return;case"select":_t("invalid",t),r=_=f=null;for(u in a)if(a.hasOwnProperty(u)&&(A=a[u],A!=null))switch(u){case"value":f=A;break;case"defaultValue":_=A;break;case"multiple":r=A;default:Ht(t,n,u,A,a,null)}n=f,a=_,t.multiple=!!r,n!=null?Zn(t,!!r,n,!1):a!=null&&Zn(t,!!r,a,!0);return;case"textarea":_t("invalid",t),f=u=r=null;for(_ in a)if(a.hasOwnProperty(_)&&(A=a[_],A!=null))switch(_){case"value":r=A;break;case"defaultValue":u=A;break;case"children":f=A;break;case"dangerouslySetInnerHTML":if(A!=null)throw Error(s(91));break;default:Ht(t,n,_,A,a,null)}on(t,r,u,f);return;case"option":for(B in a)if(a.hasOwnProperty(B)&&(r=a[B],r!=null))switch(B){case"selected":t.selected=r&&typeof r!="function"&&typeof r!="symbol";break;default:Ht(t,n,B,r,a,null)}return;case"dialog":_t("beforetoggle",t),_t("toggle",t),_t("cancel",t),_t("close",t);break;case"iframe":case"object":_t("load",t);break;case"video":case"audio":for(r=0;r<Ro.length;r++)_t(Ro[r],t);break;case"image":_t("error",t),_t("load",t);break;case"details":_t("toggle",t);break;case"embed":case"source":case"link":_t("error",t),_t("load",t);case"area":case"base":case"br":case"col":case"hr":case"keygen":case"meta":case"param":case"track":case"wbr":case"menuitem":for($ in a)if(a.hasOwnProperty($)&&(r=a[$],r!=null))switch($){case"children":case"dangerouslySetInnerHTML":throw Error(s(137,n));default:Ht(t,n,$,r,a,null)}return;default:if(Vs(n)){for(he in a)a.hasOwnProperty(he)&&(r=a[he],r!==void 0&&kf(t,n,he,r,a,void 0));return}}for(A in a)a.hasOwnProperty(A)&&(r=a[A],r!=null&&Ht(t,n,A,r,a,null))}function cy(t,n,a,r){switch(n){case"div":case"span":case"svg":case"path":case"a":case"g":case"p":case"li":break;case"input":var u=null,f=null,_=null,A=null,B=null,$=null,he=null;for(oe in a){var _e=a[oe];if(a.hasOwnProperty(oe)&&_e!=null)switch(oe){case"checked":break;case"value":break;case"defaultValue":B=_e;default:r.hasOwnProperty(oe)||Ht(t,n,oe,null,r,_e)}}for(var ae in r){var oe=r[ae];if(_e=a[ae],r.hasOwnProperty(ae)&&(oe!=null||_e!=null))switch(ae){case"type":f=oe;break;case"name":u=oe;break;case"checked":$=oe;break;case"defaultChecked":he=oe;break;case"value":_=oe;break;case"defaultValue":A=oe;break;case"children":case"dangerouslySetInnerHTML":if(oe!=null)throw Error(s(137,n));break;default:oe!==_e&&Ht(t,n,ae,oe,r,_e)}}Ln(t,_,A,B,$,he,f,u);return;case"select":oe=_=A=ae=null;for(f in a)if(B=a[f],a.hasOwnProperty(f)&&B!=null)switch(f){case"value":break;case"multiple":oe=B;default:r.hasOwnProperty(f)||Ht(t,n,f,null,r,B)}for(u in r)if(f=r[u],B=a[u],r.hasOwnProperty(u)&&(f!=null||B!=null))switch(u){case"value":ae=f;break;case"defaultValue":A=f;break;case"multiple":_=f;default:f!==B&&Ht(t,n,u,f,r,B)}n=A,a=_,r=oe,ae!=null?Zn(t,!!a,ae,!1):!!r!=!!a&&(n!=null?Zn(t,!!a,n,!0):Zn(t,!!a,a?[]:"",!1));return;case"textarea":oe=ae=null;for(A in a)if(u=a[A],a.hasOwnProperty(A)&&u!=null&&!r.hasOwnProperty(A))switch(A){case"value":break;case"children":break;default:Ht(t,n,A,null,r,u)}for(_ in r)if(u=r[_],f=a[_],r.hasOwnProperty(_)&&(u!=null||f!=null))switch(_){case"value":ae=u;break;case"defaultValue":oe=u;break;case"children":break;case"dangerouslySetInnerHTML":if(u!=null)throw Error(s(91));break;default:u!==f&&Ht(t,n,_,u,r,f)}Ot(t,ae,oe);return;case"option":for(var Ge in a)if(ae=a[Ge],a.hasOwnProperty(Ge)&&ae!=null&&!r.hasOwnProperty(Ge))switch(Ge){case"selected":t.selected=!1;break;default:Ht(t,n,Ge,null,r,ae)}for(B in r)if(ae=r[B],oe=a[B],r.hasOwnProperty(B)&&ae!==oe&&(ae!=null||oe!=null))switch(B){case"selected":t.selected=ae&&typeof ae!="function"&&typeof ae!="symbol";break;default:Ht(t,n,B,ae,r,oe)}return;case"img":case"link":case"area":case"base":case"br":case"col":case"embed":case"hr":case"keygen":case"meta":case"param":case"source":case"track":case"wbr":case"menuitem":for(var Je in a)ae=a[Je],a.hasOwnProperty(Je)&&ae!=null&&!r.hasOwnProperty(Je)&&Ht(t,n,Je,null,r,ae);for($ in r)if(ae=r[$],oe=a[$],r.hasOwnProperty($)&&ae!==oe&&(ae!=null||oe!=null))switch($){case"children":case"dangerouslySetInnerHTML":if(ae!=null)throw Error(s(137,n));break;default:Ht(t,n,$,ae,r,oe)}return;default:if(Vs(n)){for(var Gt in a)ae=a[Gt],a.hasOwnProperty(Gt)&&ae!==void 0&&!r.hasOwnProperty(Gt)&&kf(t,n,Gt,void 0,r,ae);for(he in r)ae=r[he],oe=a[he],!r.hasOwnProperty(he)||ae===oe||ae===void 0&&oe===void 0||kf(t,n,he,ae,r,oe);return}}for(var Z in a)ae=a[Z],a.hasOwnProperty(Z)&&ae!=null&&!r.hasOwnProperty(Z)&&Ht(t,n,Z,null,r,ae);for(_e in r)ae=r[_e],oe=a[_e],!r.hasOwnProperty(_e)||ae===oe||ae==null&&oe==null||Ht(t,n,_e,ae,r,oe)}function a0(t){switch(t){case"css":case"script":case"font":case"img":case"image":case"input":case"link":return!0;default:return!1}}function uy(){if(typeof performance.getEntriesByType=="function"){for(var t=0,n=0,a=performance.getEntriesByType("resource"),r=0;r<a.length;r++){var u=a[r],f=u.transferSize,_=u.initiatorType,A=u.duration;if(f&&A&&a0(_)){for(_=0,A=u.responseEnd,r+=1;r<a.length;r++){var B=a[r],$=B.startTime;if($>A)break;var he=B.transferSize,_e=B.initiatorType;he&&a0(_e)&&(B=B.responseEnd,_+=he*(B<A?1:(A-$)/(B-$)))}if(--r,n+=8*(f+_)/(u.duration/1e3),t++,10<t)break}}if(0<t)return n/t/1e6}return navigator.connection&&(t=navigator.connection.downlink,typeof t=="number")?t:5}var Xf=null,jf=null;function $l(t){return t.nodeType===9?t:t.ownerDocument}function s0(t){switch(t){case"http://www.w3.org/2000/svg":return 1;case"http://www.w3.org/1998/Math/MathML":return 2;default:return 0}}function r0(t,n){if(t===0)switch(n){case"svg":return 1;case"math":return 2;default:return 0}return t===1&&n==="foreignObject"?0:t}function Wf(t,n){return t==="textarea"||t==="noscript"||typeof n.children=="string"||typeof n.children=="number"||typeof n.children=="bigint"||typeof n.dangerouslySetInnerHTML=="object"&&n.dangerouslySetInnerHTML!==null&&n.dangerouslySetInnerHTML.__html!=null}var qf=null;function fy(){var t=window.event;return t&&t.type==="popstate"?t===qf?!1:(qf=t,!0):(qf=null,!1)}var o0=typeof setTimeout=="function"?setTimeout:void 0,dy=typeof clearTimeout=="function"?clearTimeout:void 0,l0=typeof Promise=="function"?Promise:void 0,hy=typeof queueMicrotask=="function"?queueMicrotask:typeof l0<"u"?function(t){return l0.resolve(null).then(t).catch(py)}:o0;function py(t){setTimeout(function(){throw t})}function Xa(t){return t==="head"}function c0(t,n){var a=n,r=0;do{var u=a.nextSibling;if(t.removeChild(a),u&&u.nodeType===8)if(a=u.data,a==="/$"||a==="/&"){if(r===0){t.removeChild(u),xr(n);return}r--}else if(a==="$"||a==="$?"||a==="$~"||a==="$!"||a==="&")r++;else if(a==="html")Co(t.ownerDocument.documentElement);else if(a==="head"){a=t.ownerDocument.head,Co(a);for(var f=a.firstChild;f;){var _=f.nextSibling,A=f.nodeName;f[os]||A==="SCRIPT"||A==="STYLE"||A==="LINK"&&f.rel.toLowerCase()==="stylesheet"||a.removeChild(f),f=_}}else a==="body"&&Co(t.ownerDocument.body);a=u}while(a);xr(n)}function u0(t,n){var a=t;t=0;do{var r=a.nextSibling;if(a.nodeType===1?n?(a._stashedDisplay=a.style.display,a.style.display="none"):(a.style.display=a._stashedDisplay||"",a.getAttribute("style")===""&&a.removeAttribute("style")):a.nodeType===3&&(n?(a._stashedText=a.nodeValue,a.nodeValue=""):a.nodeValue=a._stashedText||""),r&&r.nodeType===8)if(a=r.data,a==="/$"){if(t===0)break;t--}else a!=="$"&&a!=="$?"&&a!=="$~"&&a!=="$!"||t++;a=r}while(a)}function Yf(t){var n=t.firstChild;for(n&&n.nodeType===10&&(n=n.nextSibling);n;){var a=n;switch(n=n.nextSibling,a.nodeName){case"HTML":case"HEAD":case"BODY":Yf(a),qr(a);continue;case"SCRIPT":case"STYLE":continue;case"LINK":if(a.rel.toLowerCase()==="stylesheet")continue}t.removeChild(a)}}function my(t,n,a,r){for(;t.nodeType===1;){var u=a;if(t.nodeName.toLowerCase()!==n.toLowerCase()){if(!r&&(t.nodeName!=="INPUT"||t.type!=="hidden"))break}else if(r){if(!t[os])switch(n){case"meta":if(!t.hasAttribute("itemprop"))break;return t;case"link":if(f=t.getAttribute("rel"),f==="stylesheet"&&t.hasAttribute("data-precedence"))break;if(f!==u.rel||t.getAttribute("href")!==(u.href==null||u.href===""?null:u.href)||t.getAttribute("crossorigin")!==(u.crossOrigin==null?null:u.crossOrigin)||t.getAttribute("title")!==(u.title==null?null:u.title))break;return t;case"style":if(t.hasAttribute("data-precedence"))break;return t;case"script":if(f=t.getAttribute("src"),(f!==(u.src==null?null:u.src)||t.getAttribute("type")!==(u.type==null?null:u.type)||t.getAttribute("crossorigin")!==(u.crossOrigin==null?null:u.crossOrigin))&&f&&t.hasAttribute("async")&&!t.hasAttribute("itemprop"))break;return t;default:return t}}else if(n==="input"&&t.type==="hidden"){var f=u.name==null?null:""+u.name;if(u.type==="hidden"&&t.getAttribute("name")===f)return t}else return t;if(t=hi(t.nextSibling),t===null)break}return null}function gy(t,n,a){if(n==="")return null;for(;t.nodeType!==3;)if((t.nodeType!==1||t.nodeName!=="INPUT"||t.type!=="hidden")&&!a||(t=hi(t.nextSibling),t===null))return null;return t}function f0(t,n){for(;t.nodeType!==8;)if((t.nodeType!==1||t.nodeName!=="INPUT"||t.type!=="hidden")&&!n||(t=hi(t.nextSibling),t===null))return null;return t}function Zf(t){return t.data==="$?"||t.data==="$~"}function Kf(t){return t.data==="$!"||t.data==="$?"&&t.ownerDocument.readyState!=="loading"}function _y(t,n){var a=t.ownerDocument;if(t.data==="$~")t._reactRetry=n;else if(t.data!=="$?"||a.readyState!=="loading")n();else{var r=function(){n(),a.removeEventListener("DOMContentLoaded",r)};a.addEventListener("DOMContentLoaded",r),t._reactRetry=r}}function hi(t){for(;t!=null;t=t.nextSibling){var n=t.nodeType;if(n===1||n===3)break;if(n===8){if(n=t.data,n==="$"||n==="$!"||n==="$?"||n==="$~"||n==="&"||n==="F!"||n==="F")break;if(n==="/$"||n==="/&")return null}}return t}var Qf=null;function d0(t){t=t.nextSibling;for(var n=0;t;){if(t.nodeType===8){var a=t.data;if(a==="/$"||a==="/&"){if(n===0)return hi(t.nextSibling);n--}else a!=="$"&&a!=="$!"&&a!=="$?"&&a!=="$~"&&a!=="&"||n++}t=t.nextSibling}return null}function h0(t){t=t.previousSibling;for(var n=0;t;){if(t.nodeType===8){var a=t.data;if(a==="$"||a==="$!"||a==="$?"||a==="$~"||a==="&"){if(n===0)return t;n--}else a!=="/$"&&a!=="/&"||n++}t=t.previousSibling}return null}function p0(t,n,a){switch(n=$l(a),t){case"html":if(t=n.documentElement,!t)throw Error(s(452));return t;case"head":if(t=n.head,!t)throw Error(s(453));return t;case"body":if(t=n.body,!t)throw Error(s(454));return t;default:throw Error(s(451))}}function Co(t){for(var n=t.attributes;n.length;)t.removeAttributeNode(n[0]);qr(t)}var pi=new Map,m0=new Set;function ec(t){return typeof t.getRootNode=="function"?t.getRootNode():t.nodeType===9?t:t.ownerDocument}var da=z.d;z.d={f:vy,r:xy,D:yy,C:Sy,L:by,m:My,X:Ty,S:Ey,M:Ay};function vy(){var t=da.f(),n=jl();return t||n}function xy(t){var n=Aa(t);n!==null&&n.tag===5&&n.type==="form"?Um(n):da.r(t)}var gr=typeof document>"u"?null:document;function g0(t,n,a){var r=gr;if(r&&typeof n=="string"&&n){var u=it(n);u='link[rel="'+t+'"][href="'+u+'"]',typeof a=="string"&&(u+='[crossorigin="'+a+'"]'),m0.has(u)||(m0.add(u),t={rel:t,crossOrigin:a,href:n},r.querySelector(u)===null&&(n=r.createElement("link"),Tn(n,"link",t),j(n),r.head.appendChild(n)))}}function yy(t){da.D(t),g0("dns-prefetch",t,null)}function Sy(t,n){da.C(t,n),g0("preconnect",t,n)}function by(t,n,a){da.L(t,n,a);var r=gr;if(r&&t&&n){var u='link[rel="preload"][as="'+it(n)+'"]';n==="image"&&a&&a.imageSrcSet?(u+='[imagesrcset="'+it(a.imageSrcSet)+'"]',typeof a.imageSizes=="string"&&(u+='[imagesizes="'+it(a.imageSizes)+'"]')):u+='[href="'+it(t)+'"]';var f=u;switch(n){case"style":f=_r(t);break;case"script":f=vr(t)}pi.has(f)||(t=y({rel:"preload",href:n==="image"&&a&&a.imageSrcSet?void 0:t,as:n},a),pi.set(f,t),r.querySelector(u)!==null||n==="style"&&r.querySelector(Do(f))||n==="script"&&r.querySelector(No(f))||(n=r.createElement("link"),Tn(n,"link",t),j(n),r.head.appendChild(n)))}}function My(t,n){da.m(t,n);var a=gr;if(a&&t){var r=n&&typeof n.as=="string"?n.as:"script",u='link[rel="modulepreload"][as="'+it(r)+'"][href="'+it(t)+'"]',f=u;switch(r){case"audioworklet":case"paintworklet":case"serviceworker":case"sharedworker":case"worker":case"script":f=vr(t)}if(!pi.has(f)&&(t=y({rel:"modulepreload",href:t},n),pi.set(f,t),a.querySelector(u)===null)){switch(r){case"audioworklet":case"paintworklet":case"serviceworker":case"sharedworker":case"worker":case"script":if(a.querySelector(No(f)))return}r=a.createElement("link"),Tn(r,"link",t),j(r),a.head.appendChild(r)}}}function Ey(t,n,a){da.S(t,n,a);var r=gr;if(r&&t){var u=R(r).hoistableStyles,f=_r(t);n=n||"default";var _=u.get(f);if(!_){var A={loading:0,preload:null};if(_=r.querySelector(Do(f)))A.loading=5;else{t=y({rel:"stylesheet",href:t,"data-precedence":n},a),(a=pi.get(f))&&Jf(t,a);var B=_=r.createElement("link");j(B),Tn(B,"link",t),B._p=new Promise(function($,he){B.onload=$,B.onerror=he}),B.addEventListener("load",function(){A.loading|=1}),B.addEventListener("error",function(){A.loading|=2}),A.loading|=4,tc(_,n,r)}_={type:"stylesheet",instance:_,count:1,state:A},u.set(f,_)}}}function Ty(t,n){da.X(t,n);var a=gr;if(a&&t){var r=R(a).hoistableScripts,u=vr(t),f=r.get(u);f||(f=a.querySelector(No(u)),f||(t=y({src:t,async:!0},n),(n=pi.get(u))&&$f(t,n),f=a.createElement("script"),j(f),Tn(f,"link",t),a.head.appendChild(f)),f={type:"script",instance:f,count:1,state:null},r.set(u,f))}}function Ay(t,n){da.M(t,n);var a=gr;if(a&&t){var r=R(a).hoistableScripts,u=vr(t),f=r.get(u);f||(f=a.querySelector(No(u)),f||(t=y({src:t,async:!0,type:"module"},n),(n=pi.get(u))&&$f(t,n),f=a.createElement("script"),j(f),Tn(f,"link",t),a.head.appendChild(f)),f={type:"script",instance:f,count:1,state:null},r.set(u,f))}}function _0(t,n,a,r){var u=(u=ie.current)?ec(u):null;if(!u)throw Error(s(446));switch(t){case"meta":case"title":return null;case"style":return typeof a.precedence=="string"&&typeof a.href=="string"?(n=_r(a.href),a=R(u).hoistableStyles,r=a.get(n),r||(r={type:"style",instance:null,count:0,state:null},a.set(n,r)),r):{type:"void",instance:null,count:0,state:null};case"link":if(a.rel==="stylesheet"&&typeof a.href=="string"&&typeof a.precedence=="string"){t=_r(a.href);var f=R(u).hoistableStyles,_=f.get(t);if(_||(u=u.ownerDocument||u,_={type:"stylesheet",instance:null,count:0,state:{loading:0,preload:null}},f.set(t,_),(f=u.querySelector(Do(t)))&&!f._p&&(_.instance=f,_.state.loading=5),pi.has(t)||(a={rel:"preload",as:"style",href:a.href,crossOrigin:a.crossOrigin,integrity:a.integrity,media:a.media,hrefLang:a.hrefLang,referrerPolicy:a.referrerPolicy},pi.set(t,a),f||Ry(u,t,a,_.state))),n&&r===null)throw Error(s(528,""));return _}if(n&&r!==null)throw Error(s(529,""));return null;case"script":return n=a.async,a=a.src,typeof a=="string"&&n&&typeof n!="function"&&typeof n!="symbol"?(n=vr(a),a=R(u).hoistableScripts,r=a.get(n),r||(r={type:"script",instance:null,count:0,state:null},a.set(n,r)),r):{type:"void",instance:null,count:0,state:null};default:throw Error(s(444,t))}}function _r(t){return'href="'+it(t)+'"'}function Do(t){return'link[rel="stylesheet"]['+t+"]"}function v0(t){return y({},t,{"data-precedence":t.precedence,precedence:null})}function Ry(t,n,a,r){t.querySelector('link[rel="preload"][as="style"]['+n+"]")?r.loading=1:(n=t.createElement("link"),r.preload=n,n.addEventListener("load",function(){return r.loading|=1}),n.addEventListener("error",function(){return r.loading|=2}),Tn(n,"link",a),j(n),t.head.appendChild(n))}function vr(t){return'[src="'+it(t)+'"]'}function No(t){return"script[async]"+t}function x0(t,n,a){if(n.count++,n.instance===null)switch(n.type){case"style":var r=t.querySelector('style[data-href~="'+it(a.href)+'"]');if(r)return n.instance=r,j(r),r;var u=y({},a,{"data-href":a.href,"data-precedence":a.precedence,href:null,precedence:null});return r=(t.ownerDocument||t).createElement("style"),j(r),Tn(r,"style",u),tc(r,a.precedence,t),n.instance=r;case"stylesheet":u=_r(a.href);var f=t.querySelector(Do(u));if(f)return n.state.loading|=4,n.instance=f,j(f),f;r=v0(a),(u=pi.get(u))&&Jf(r,u),f=(t.ownerDocument||t).createElement("link"),j(f);var _=f;return _._p=new Promise(function(A,B){_.onload=A,_.onerror=B}),Tn(f,"link",r),n.state.loading|=4,tc(f,a.precedence,t),n.instance=f;case"script":return f=vr(a.src),(u=t.querySelector(No(f)))?(n.instance=u,j(u),u):(r=a,(u=pi.get(f))&&(r=y({},a),$f(r,u)),t=t.ownerDocument||t,u=t.createElement("script"),j(u),Tn(u,"link",r),t.head.appendChild(u),n.instance=u);case"void":return null;default:throw Error(s(443,n.type))}else n.type==="stylesheet"&&(n.state.loading&4)===0&&(r=n.instance,n.state.loading|=4,tc(r,a.precedence,t));return n.instance}function tc(t,n,a){for(var r=a.querySelectorAll('link[rel="stylesheet"][data-precedence],style[data-precedence]'),u=r.length?r[r.length-1]:null,f=u,_=0;_<r.length;_++){var A=r[_];if(A.dataset.precedence===n)f=A;else if(f!==u)break}f?f.parentNode.insertBefore(t,f.nextSibling):(n=a.nodeType===9?a.head:a,n.insertBefore(t,n.firstChild))}function Jf(t,n){t.crossOrigin==null&&(t.crossOrigin=n.crossOrigin),t.referrerPolicy==null&&(t.referrerPolicy=n.referrerPolicy),t.title==null&&(t.title=n.title)}function $f(t,n){t.crossOrigin==null&&(t.crossOrigin=n.crossOrigin),t.referrerPolicy==null&&(t.referrerPolicy=n.referrerPolicy),t.integrity==null&&(t.integrity=n.integrity)}var nc=null;function y0(t,n,a){if(nc===null){var r=new Map,u=nc=new Map;u.set(a,r)}else u=nc,r=u.get(a),r||(r=new Map,u.set(a,r));if(r.has(t))return r;for(r.set(t,null),a=a.getElementsByTagName(t),u=0;u<a.length;u++){var f=a[u];if(!(f[os]||f[rn]||t==="link"&&f.getAttribute("rel")==="stylesheet")&&f.namespaceURI!=="http://www.w3.org/2000/svg"){var _=f.getAttribute(n)||"";_=t+_;var A=r.get(_);A?A.push(f):r.set(_,[f])}}return r}function S0(t,n,a){t=t.ownerDocument||t,t.head.insertBefore(a,n==="title"?t.querySelector("head > title"):null)}function wy(t,n,a){if(a===1||n.itemProp!=null)return!1;switch(t){case"meta":case"title":return!0;case"style":if(typeof n.precedence!="string"||typeof n.href!="string"||n.href==="")break;return!0;case"link":if(typeof n.rel!="string"||typeof n.href!="string"||n.href===""||n.onLoad||n.onError)break;switch(n.rel){case"stylesheet":return t=n.disabled,typeof n.precedence=="string"&&t==null;default:return!0}case"script":if(n.async&&typeof n.async!="function"&&typeof n.async!="symbol"&&!n.onLoad&&!n.onError&&n.src&&typeof n.src=="string")return!0}return!1}function b0(t){return!(t.type==="stylesheet"&&(t.state.loading&3)===0)}function Cy(t,n,a,r){if(a.type==="stylesheet"&&(typeof r.media!="string"||matchMedia(r.media).matches!==!1)&&(a.state.loading&4)===0){if(a.instance===null){var u=_r(r.href),f=n.querySelector(Do(u));if(f){n=f._p,n!==null&&typeof n=="object"&&typeof n.then=="function"&&(t.count++,t=ic.bind(t),n.then(t,t)),a.state.loading|=4,a.instance=f,j(f);return}f=n.ownerDocument||n,r=v0(r),(u=pi.get(u))&&Jf(r,u),f=f.createElement("link"),j(f);var _=f;_._p=new Promise(function(A,B){_.onload=A,_.onerror=B}),Tn(f,"link",r),a.instance=f}t.stylesheets===null&&(t.stylesheets=new Map),t.stylesheets.set(a,n),(n=a.state.preload)&&(a.state.loading&3)===0&&(t.count++,a=ic.bind(t),n.addEventListener("load",a),n.addEventListener("error",a))}}var ed=0;function Dy(t,n){return t.stylesheets&&t.count===0&&sc(t,t.stylesheets),0<t.count||0<t.imgCount?function(a){var r=setTimeout(function(){if(t.stylesheets&&sc(t,t.stylesheets),t.unsuspend){var f=t.unsuspend;t.unsuspend=null,f()}},6e4+n);0<t.imgBytes&&ed===0&&(ed=62500*uy());var u=setTimeout(function(){if(t.waitingForImages=!1,t.count===0&&(t.stylesheets&&sc(t,t.stylesheets),t.unsuspend)){var f=t.unsuspend;t.unsuspend=null,f()}},(t.imgBytes>ed?50:800)+n);return t.unsuspend=a,function(){t.unsuspend=null,clearTimeout(r),clearTimeout(u)}}:null}function ic(){if(this.count--,this.count===0&&(this.imgCount===0||!this.waitingForImages)){if(this.stylesheets)sc(this,this.stylesheets);else if(this.unsuspend){var t=this.unsuspend;this.unsuspend=null,t()}}}var ac=null;function sc(t,n){t.stylesheets=null,t.unsuspend!==null&&(t.count++,ac=new Map,n.forEach(Ny,t),ac=null,ic.call(t))}function Ny(t,n){if(!(n.state.loading&4)){var a=ac.get(t);if(a)var r=a.get(null);else{a=new Map,ac.set(t,a);for(var u=t.querySelectorAll("link[data-precedence],style[data-precedence]"),f=0;f<u.length;f++){var _=u[f];(_.nodeName==="LINK"||_.getAttribute("media")!=="not all")&&(a.set(_.dataset.precedence,_),r=_)}r&&a.set(null,r)}u=n.instance,_=u.getAttribute("data-precedence"),f=a.get(_)||r,f===r&&a.set(null,u),a.set(_,u),this.count++,r=ic.bind(this),u.addEventListener("load",r),u.addEventListener("error",r),f?f.parentNode.insertBefore(u,f.nextSibling):(t=t.nodeType===9?t.head:t,t.insertBefore(u,t.firstChild)),n.state.loading|=4}}var Uo={$$typeof:U,Provider:null,Consumer:null,_currentValue:ce,_currentValue2:ce,_threadCount:0};function Uy(t,n,a,r,u,f,_,A,B){this.tag=1,this.containerInfo=t,this.pingCache=this.current=this.pendingChildren=null,this.timeoutHandle=-1,this.callbackNode=this.next=this.pendingContext=this.context=this.cancelPendingCommit=null,this.callbackPriority=0,this.expirationTimes=Et(-1),this.entangledLanes=this.shellSuspendCounter=this.errorRecoveryDisabledLanes=this.expiredLanes=this.warmLanes=this.pingedLanes=this.suspendedLanes=this.pendingLanes=0,this.entanglements=Et(0),this.hiddenUpdates=Et(null),this.identifierPrefix=r,this.onUncaughtError=u,this.onCaughtError=f,this.onRecoverableError=_,this.pooledCache=null,this.pooledCacheLanes=0,this.formState=B,this.incompleteTransitions=new Map}function M0(t,n,a,r,u,f,_,A,B,$,he,_e){return t=new Uy(t,n,a,_,B,$,he,_e,A),n=1,f===!0&&(n|=24),f=Qn(3,null,null,n),t.current=f,f.stateNode=t,n=Uu(),n.refCount++,t.pooledCache=n,n.refCount++,f.memoizedState={element:r,isDehydrated:a,cache:n},Iu(f),t}function E0(t){return t?(t=Zs,t):Zs}function T0(t,n,a,r,u,f){u=E0(u),r.context===null?r.context=u:r.pendingContext=u,r=La(n),r.payload={element:a},f=f===void 0?null:f,f!==null&&(r.callback=f),a=Oa(t,r,n),a!==null&&(Xn(a,t,n),co(a,t,n))}function A0(t,n){if(t=t.memoizedState,t!==null&&t.dehydrated!==null){var a=t.retryLane;t.retryLane=a!==0&&a<n?a:n}}function td(t,n){A0(t,n),(t=t.alternate)&&A0(t,n)}function R0(t){if(t.tag===13||t.tag===31){var n=ds(t,67108864);n!==null&&Xn(n,t,67108864),td(t,67108864)}}function w0(t){if(t.tag===13||t.tag===31){var n=ni();n=Bs(n);var a=ds(t,n);a!==null&&Xn(a,t,n),td(t,n)}}var rc=!0;function Ly(t,n,a,r){var u=P.T;P.T=null;var f=z.p;try{z.p=2,nd(t,n,a,r)}finally{z.p=f,P.T=u}}function Oy(t,n,a,r){var u=P.T;P.T=null;var f=z.p;try{z.p=8,nd(t,n,a,r)}finally{z.p=f,P.T=u}}function nd(t,n,a,r){if(rc){var u=id(r);if(u===null)Vf(t,n,r,oc,a),D0(t,r);else if(Iy(u,t,n,a,r))r.stopPropagation();else if(D0(t,r),n&4&&-1<Py.indexOf(t)){for(;u!==null;){var f=Aa(u);if(f!==null)switch(f.tag){case 3:if(f=f.stateNode,f.current.memoizedState.isDehydrated){var _=be(f.pendingLanes);if(_!==0){var A=f;for(A.pendingLanes|=2,A.entangledLanes|=2;_;){var B=1<<31-Le(_);A.entanglements[1]|=B,_&=~B}Ii(f),(Ut&6)===0&&(kl=M()+500,Ao(0))}}break;case 31:case 13:A=ds(f,2),A!==null&&Xn(A,f,2),jl(),td(f,2)}if(f=id(r),f===null&&Vf(t,n,r,oc,a),f===u)break;u=f}u!==null&&r.stopPropagation()}else Vf(t,n,r,null,a)}}function id(t){return t=au(t),ad(t)}var oc=null;function ad(t){if(oc=null,t=Ta(t),t!==null){var n=c(t);if(n===null)t=null;else{var a=n.tag;if(a===13){if(t=d(n),t!==null)return t;t=null}else if(a===31){if(t=p(n),t!==null)return t;t=null}else if(a===3){if(n.stateNode.current.memoizedState.isDehydrated)return n.tag===3?n.stateNode.containerInfo:null;t=null}else n!==t&&(t=null)}}return oc=t,null}function C0(t){switch(t){case"beforetoggle":case"cancel":case"click":case"close":case"contextmenu":case"copy":case"cut":case"auxclick":case"dblclick":case"dragend":case"dragstart":case"drop":case"focusin":case"focusout":case"input":case"invalid":case"keydown":case"keypress":case"keyup":case"mousedown":case"mouseup":case"paste":case"pause":case"play":case"pointercancel":case"pointerdown":case"pointerup":case"ratechange":case"reset":case"resize":case"seeked":case"submit":case"toggle":case"touchcancel":case"touchend":case"touchstart":case"volumechange":case"change":case"selectionchange":case"textInput":case"compositionstart":case"compositionend":case"compositionupdate":case"beforeblur":case"afterblur":case"beforeinput":case"blur":case"fullscreenchange":case"focus":case"hashchange":case"popstate":case"select":case"selectstart":return 2;case"drag":case"dragenter":case"dragexit":case"dragleave":case"dragover":case"mousemove":case"mouseout":case"mouseover":case"pointermove":case"pointerout":case"pointerover":case"scroll":case"touchmove":case"wheel":case"mouseenter":case"mouseleave":case"pointerenter":case"pointerleave":return 8;case"message":switch(q()){case me:return 2;case ye:return 8;case de:case Xe:return 32;case Ce:return 268435456;default:return 32}default:return 32}}var sd=!1,ja=null,Wa=null,qa=null,Lo=new Map,Oo=new Map,Ya=[],Py="mousedown mouseup touchcancel touchend touchstart auxclick dblclick pointercancel pointerdown pointerup dragend dragstart drop compositionend compositionstart keydown keypress keyup input textInput copy cut paste click change contextmenu reset".split(" ");function D0(t,n){switch(t){case"focusin":case"focusout":ja=null;break;case"dragenter":case"dragleave":Wa=null;break;case"mouseover":case"mouseout":qa=null;break;case"pointerover":case"pointerout":Lo.delete(n.pointerId);break;case"gotpointercapture":case"lostpointercapture":Oo.delete(n.pointerId)}}function Po(t,n,a,r,u,f){return t===null||t.nativeEvent!==f?(t={blockedOn:n,domEventName:a,eventSystemFlags:r,nativeEvent:f,targetContainers:[u]},n!==null&&(n=Aa(n),n!==null&&R0(n)),t):(t.eventSystemFlags|=r,n=t.targetContainers,u!==null&&n.indexOf(u)===-1&&n.push(u),t)}function Iy(t,n,a,r,u){switch(n){case"focusin":return ja=Po(ja,t,n,a,r,u),!0;case"dragenter":return Wa=Po(Wa,t,n,a,r,u),!0;case"mouseover":return qa=Po(qa,t,n,a,r,u),!0;case"pointerover":var f=u.pointerId;return Lo.set(f,Po(Lo.get(f)||null,t,n,a,r,u)),!0;case"gotpointercapture":return f=u.pointerId,Oo.set(f,Po(Oo.get(f)||null,t,n,a,r,u)),!0}return!1}function N0(t){var n=Ta(t.target);if(n!==null){var a=c(n);if(a!==null){if(n=a.tag,n===13){if(n=d(a),n!==null){t.blockedOn=n,Gs(t.priority,function(){w0(a)});return}}else if(n===31){if(n=p(a),n!==null){t.blockedOn=n,Gs(t.priority,function(){w0(a)});return}}else if(n===3&&a.stateNode.current.memoizedState.isDehydrated){t.blockedOn=a.tag===3?a.stateNode.containerInfo:null;return}}}t.blockedOn=null}function lc(t){if(t.blockedOn!==null)return!1;for(var n=t.targetContainers;0<n.length;){var a=id(t.nativeEvent);if(a===null){a=t.nativeEvent;var r=new a.constructor(a.type,a);iu=r,a.target.dispatchEvent(r),iu=null}else return n=Aa(a),n!==null&&R0(n),t.blockedOn=a,!1;n.shift()}return!0}function U0(t,n,a){lc(t)&&a.delete(n)}function Fy(){sd=!1,ja!==null&&lc(ja)&&(ja=null),Wa!==null&&lc(Wa)&&(Wa=null),qa!==null&&lc(qa)&&(qa=null),Lo.forEach(U0),Oo.forEach(U0)}function cc(t,n){t.blockedOn===n&&(t.blockedOn=null,sd||(sd=!0,o.unstable_scheduleCallback(o.unstable_NormalPriority,Fy)))}var uc=null;function L0(t){uc!==t&&(uc=t,o.unstable_scheduleCallback(o.unstable_NormalPriority,function(){uc===t&&(uc=null);for(var n=0;n<t.length;n+=3){var a=t[n],r=t[n+1],u=t[n+2];if(typeof r!="function"){if(ad(r||a)===null)continue;break}var f=Aa(a);f!==null&&(t.splice(n,3),n-=3,nf(f,{pending:!0,data:u,method:a.method,action:r},r,u))}}))}function xr(t){function n(B){return cc(B,t)}ja!==null&&cc(ja,t),Wa!==null&&cc(Wa,t),qa!==null&&cc(qa,t),Lo.forEach(n),Oo.forEach(n);for(var a=0;a<Ya.length;a++){var r=Ya[a];r.blockedOn===t&&(r.blockedOn=null)}for(;0<Ya.length&&(a=Ya[0],a.blockedOn===null);)N0(a),a.blockedOn===null&&Ya.shift();if(a=(t.ownerDocument||t).$$reactFormReplay,a!=null)for(r=0;r<a.length;r+=3){var u=a[r],f=a[r+1],_=u[pn]||null;if(typeof f=="function")_||L0(a);else if(_){var A=null;if(f&&f.hasAttribute("formAction")){if(u=f,_=f[pn]||null)A=_.formAction;else if(ad(u)!==null)continue}else A=_.action;typeof A=="function"?a[r+1]=A:(a.splice(r,3),r-=3),L0(a)}}}function O0(){function t(f){f.canIntercept&&f.info==="react-transition"&&f.intercept({handler:function(){return new Promise(function(_){return u=_})},focusReset:"manual",scroll:"manual"})}function n(){u!==null&&(u(),u=null),r||setTimeout(a,20)}function a(){if(!r&&!navigation.transition){var f=navigation.currentEntry;f&&f.url!=null&&navigation.navigate(f.url,{state:f.getState(),info:"react-transition",history:"replace"})}}if(typeof navigation=="object"){var r=!1,u=null;return navigation.addEventListener("navigate",t),navigation.addEventListener("navigatesuccess",n),navigation.addEventListener("navigateerror",n),setTimeout(a,100),function(){r=!0,navigation.removeEventListener("navigate",t),navigation.removeEventListener("navigatesuccess",n),navigation.removeEventListener("navigateerror",n),u!==null&&(u(),u=null)}}}function rd(t){this._internalRoot=t}fc.prototype.render=rd.prototype.render=function(t){var n=this._internalRoot;if(n===null)throw Error(s(409));var a=n.current,r=ni();T0(a,r,t,n,null,null)},fc.prototype.unmount=rd.prototype.unmount=function(){var t=this._internalRoot;if(t!==null){this._internalRoot=null;var n=t.containerInfo;T0(t.current,2,null,t,null,null),jl(),n[Yi]=null}};function fc(t){this._internalRoot=t}fc.prototype.unstable_scheduleHydration=function(t){if(t){var n=Ui();t={blockedOn:null,target:t,priority:n};for(var a=0;a<Ya.length&&n!==0&&n<Ya[a].priority;a++);Ya.splice(a,0,t),a===0&&N0(t)}};var P0=e.version;if(P0!=="19.2.4")throw Error(s(527,P0,"19.2.4"));z.findDOMNode=function(t){var n=t._reactInternals;if(n===void 0)throw typeof t.render=="function"?Error(s(188)):(t=Object.keys(t).join(","),Error(s(268,t)));return t=h(n),t=t!==null?v(t):null,t=t===null?null:t.stateNode,t};var zy={bundleType:0,version:"19.2.4",rendererPackageName:"react-dom",currentDispatcherRef:P,reconcilerVersion:"19.2.4"};if(typeof __REACT_DEVTOOLS_GLOBAL_HOOK__<"u"){var dc=__REACT_DEVTOOLS_GLOBAL_HOOK__;if(!dc.isDisabled&&dc.supportsFiber)try{Me=dc.inject(zy),Se=dc}catch{}}return Fo.createRoot=function(t,n){if(!l(t))throw Error(s(299));var a=!1,r="",u=Vm,f=km,_=Xm;return n!=null&&(n.unstable_strictMode===!0&&(a=!0),n.identifierPrefix!==void 0&&(r=n.identifierPrefix),n.onUncaughtError!==void 0&&(u=n.onUncaughtError),n.onCaughtError!==void 0&&(f=n.onCaughtError),n.onRecoverableError!==void 0&&(_=n.onRecoverableError)),n=M0(t,1,!1,null,null,a,r,null,u,f,_,O0),t[Yi]=n.current,Gf(t),new rd(n)},Fo.hydrateRoot=function(t,n,a){if(!l(t))throw Error(s(299));var r=!1,u="",f=Vm,_=km,A=Xm,B=null;return a!=null&&(a.unstable_strictMode===!0&&(r=!0),a.identifierPrefix!==void 0&&(u=a.identifierPrefix),a.onUncaughtError!==void 0&&(f=a.onUncaughtError),a.onCaughtError!==void 0&&(_=a.onCaughtError),a.onRecoverableError!==void 0&&(A=a.onRecoverableError),a.formState!==void 0&&(B=a.formState)),n=M0(t,1,!0,n,a??null,r,u,B,f,_,A,O0),n.context=E0(null),a=n.current,r=ni(),r=Bs(r),u=La(r),u.callback=null,Oa(a,u,r),a=r,n.current.lanes=a,Nn(n,a),Ii(n),t[Yi]=n.current,Gf(t),new fc(n)},Fo.version="19.2.4",Fo}var j0;function Zy(){if(j0)return cd.exports;j0=1;function o(){if(!(typeof __REACT_DEVTOOLS_GLOBAL_HOOK__>"u"||typeof __REACT_DEVTOOLS_GLOBAL_HOOK__.checkDCE!="function"))try{__REACT_DEVTOOLS_GLOBAL_HOOK__.checkDCE(o)}catch(e){console.error(e)}}return o(),cd.exports=Yy(),cd.exports}var Ky=Zy();const Qy=K_(Ky);/**
 * @license
 * Copyright 2010-2026 Three.js Authors
 * SPDX-License-Identifier: MIT
 */const kh="183",Ir={ROTATE:0,DOLLY:1,PAN:2},Or={ROTATE:0,PAN:1,DOLLY_PAN:2,DOLLY_ROTATE:3},Jy=0,W0=1,$y=2,Gc=1,eS=2,jo=3,ss=0,qn=1,va=2,ya=0,Fr=1,qd=2,q0=3,Y0=4,tS=5,Us=100,nS=101,iS=102,aS=103,sS=104,rS=200,oS=201,lS=202,cS=203,Yd=204,Zd=205,uS=206,fS=207,dS=208,hS=209,pS=210,mS=211,gS=212,_S=213,vS=214,Kd=0,Qd=1,Jd=2,Br=3,$d=4,eh=5,th=6,nh=7,Q_=0,xS=1,yS=2,Vi=0,J_=1,$_=2,ev=3,tv=4,nv=5,iv=6,av=7,sv=300,Is=301,Hr=302,hd=303,pd=304,Jc=306,ih=1e3,xa=1001,ah=1002,An=1003,SS=1004,hc=1005,Dn=1006,md=1007,Os=1008,ri=1009,rv=1010,ov=1011,qo=1012,Xh=1013,Xi=1014,Hi=1015,ba=1016,jh=1017,Wh=1018,Yo=1020,lv=35902,cv=35899,uv=1021,fv=1022,Di=1023,Ma=1026,Ps=1027,dv=1028,qh=1029,Gr=1030,Yh=1031,Zh=1033,Vc=33776,kc=33777,Xc=33778,jc=33779,sh=35840,rh=35841,oh=35842,lh=35843,ch=36196,uh=37492,fh=37496,dh=37488,hh=37489,ph=37490,mh=37491,gh=37808,_h=37809,vh=37810,xh=37811,yh=37812,Sh=37813,bh=37814,Mh=37815,Eh=37816,Th=37817,Ah=37818,Rh=37819,wh=37820,Ch=37821,Dh=36492,Nh=36494,Uh=36495,Lh=36283,Oh=36284,Ph=36285,Ih=36286,bS=3200,hv=0,MS=1,is="",gi="srgb",Vr="srgb-linear",Yc="linear",zt="srgb",yr=7680,Z0=519,ES=512,TS=513,AS=514,Kh=515,RS=516,wS=517,Qh=518,CS=519,K0=35044,Q0="300 es",Gi=2e3,Zo=2001;function DS(o){for(let e=o.length-1;e>=0;--e)if(o[e]>=65535)return!0;return!1}function Zc(o){return document.createElementNS("http://www.w3.org/1999/xhtml",o)}function NS(){const o=Zc("canvas");return o.style.display="block",o}const J0={};function $0(...o){const e="THREE."+o.shift();console.log(e,...o)}function pv(o){const e=o[0];if(typeof e=="string"&&e.startsWith("TSL:")){const i=o[1];i&&i.isStackTrace?o[0]+=" "+i.getLocation():o[1]='Stack trace not available. Enable "THREE.Node.captureStackTrace" to capture stack traces.'}return o}function at(...o){o=pv(o);const e="THREE."+o.shift();{const i=o[0];i&&i.isStackTrace?console.warn(i.getError(e)):console.warn(e,...o)}}function Dt(...o){o=pv(o);const e="THREE."+o.shift();{const i=o[0];i&&i.isStackTrace?console.error(i.getError(e)):console.error(e,...o)}}function Kc(...o){const e=o.join(" ");e in J0||(J0[e]=!0,at(...o))}function US(o,e,i){return new Promise(function(s,l){function c(){switch(o.clientWaitSync(e,o.SYNC_FLUSH_COMMANDS_BIT,0)){case o.WAIT_FAILED:l();break;case o.TIMEOUT_EXPIRED:setTimeout(c,i);break;default:s()}}setTimeout(c,i)})}const LS={[Kd]:Qd,[Jd]:th,[$d]:nh,[Br]:eh,[Qd]:Kd,[th]:Jd,[nh]:$d,[eh]:Br};class Fs{addEventListener(e,i){this._listeners===void 0&&(this._listeners={});const s=this._listeners;s[e]===void 0&&(s[e]=[]),s[e].indexOf(i)===-1&&s[e].push(i)}hasEventListener(e,i){const s=this._listeners;return s===void 0?!1:s[e]!==void 0&&s[e].indexOf(i)!==-1}removeEventListener(e,i){const s=this._listeners;if(s===void 0)return;const l=s[e];if(l!==void 0){const c=l.indexOf(i);c!==-1&&l.splice(c,1)}}dispatchEvent(e){const i=this._listeners;if(i===void 0)return;const s=i[e.type];if(s!==void 0){e.target=this;const l=s.slice(0);for(let c=0,d=l.length;c<d;c++)l[c].call(this,e);e.target=null}}}const wn=["00","01","02","03","04","05","06","07","08","09","0a","0b","0c","0d","0e","0f","10","11","12","13","14","15","16","17","18","19","1a","1b","1c","1d","1e","1f","20","21","22","23","24","25","26","27","28","29","2a","2b","2c","2d","2e","2f","30","31","32","33","34","35","36","37","38","39","3a","3b","3c","3d","3e","3f","40","41","42","43","44","45","46","47","48","49","4a","4b","4c","4d","4e","4f","50","51","52","53","54","55","56","57","58","59","5a","5b","5c","5d","5e","5f","60","61","62","63","64","65","66","67","68","69","6a","6b","6c","6d","6e","6f","70","71","72","73","74","75","76","77","78","79","7a","7b","7c","7d","7e","7f","80","81","82","83","84","85","86","87","88","89","8a","8b","8c","8d","8e","8f","90","91","92","93","94","95","96","97","98","99","9a","9b","9c","9d","9e","9f","a0","a1","a2","a3","a4","a5","a6","a7","a8","a9","aa","ab","ac","ad","ae","af","b0","b1","b2","b3","b4","b5","b6","b7","b8","b9","ba","bb","bc","bd","be","bf","c0","c1","c2","c3","c4","c5","c6","c7","c8","c9","ca","cb","cc","cd","ce","cf","d0","d1","d2","d3","d4","d5","d6","d7","d8","d9","da","db","dc","dd","de","df","e0","e1","e2","e3","e4","e5","e6","e7","e8","e9","ea","eb","ec","ed","ee","ef","f0","f1","f2","f3","f4","f5","f6","f7","f8","f9","fa","fb","fc","fd","fe","ff"],Wc=Math.PI/180,Fh=180/Math.PI;function Qo(){const o=Math.random()*4294967295|0,e=Math.random()*4294967295|0,i=Math.random()*4294967295|0,s=Math.random()*4294967295|0;return(wn[o&255]+wn[o>>8&255]+wn[o>>16&255]+wn[o>>24&255]+"-"+wn[e&255]+wn[e>>8&255]+"-"+wn[e>>16&15|64]+wn[e>>24&255]+"-"+wn[i&63|128]+wn[i>>8&255]+"-"+wn[i>>16&255]+wn[i>>24&255]+wn[s&255]+wn[s>>8&255]+wn[s>>16&255]+wn[s>>24&255]).toLowerCase()}function vt(o,e,i){return Math.max(e,Math.min(i,o))}function OS(o,e){return(o%e+e)%e}function gd(o,e,i){return(1-i)*o+i*e}function zo(o,e){switch(e.constructor){case Float32Array:return o;case Uint32Array:return o/4294967295;case Uint16Array:return o/65535;case Uint8Array:return o/255;case Int32Array:return Math.max(o/2147483647,-1);case Int16Array:return Math.max(o/32767,-1);case Int8Array:return Math.max(o/127,-1);default:throw new Error("Invalid component type.")}}function jn(o,e){switch(e.constructor){case Float32Array:return o;case Uint32Array:return Math.round(o*4294967295);case Uint16Array:return Math.round(o*65535);case Uint8Array:return Math.round(o*255);case Int32Array:return Math.round(o*2147483647);case Int16Array:return Math.round(o*32767);case Int8Array:return Math.round(o*127);default:throw new Error("Invalid component type.")}}const PS={DEG2RAD:Wc};class ct{constructor(e=0,i=0){ct.prototype.isVector2=!0,this.x=e,this.y=i}get width(){return this.x}set width(e){this.x=e}get height(){return this.y}set height(e){this.y=e}set(e,i){return this.x=e,this.y=i,this}setScalar(e){return this.x=e,this.y=e,this}setX(e){return this.x=e,this}setY(e){return this.y=e,this}setComponent(e,i){switch(e){case 0:this.x=i;break;case 1:this.y=i;break;default:throw new Error("index is out of range: "+e)}return this}getComponent(e){switch(e){case 0:return this.x;case 1:return this.y;default:throw new Error("index is out of range: "+e)}}clone(){return new this.constructor(this.x,this.y)}copy(e){return this.x=e.x,this.y=e.y,this}add(e){return this.x+=e.x,this.y+=e.y,this}addScalar(e){return this.x+=e,this.y+=e,this}addVectors(e,i){return this.x=e.x+i.x,this.y=e.y+i.y,this}addScaledVector(e,i){return this.x+=e.x*i,this.y+=e.y*i,this}sub(e){return this.x-=e.x,this.y-=e.y,this}subScalar(e){return this.x-=e,this.y-=e,this}subVectors(e,i){return this.x=e.x-i.x,this.y=e.y-i.y,this}multiply(e){return this.x*=e.x,this.y*=e.y,this}multiplyScalar(e){return this.x*=e,this.y*=e,this}divide(e){return this.x/=e.x,this.y/=e.y,this}divideScalar(e){return this.multiplyScalar(1/e)}applyMatrix3(e){const i=this.x,s=this.y,l=e.elements;return this.x=l[0]*i+l[3]*s+l[6],this.y=l[1]*i+l[4]*s+l[7],this}min(e){return this.x=Math.min(this.x,e.x),this.y=Math.min(this.y,e.y),this}max(e){return this.x=Math.max(this.x,e.x),this.y=Math.max(this.y,e.y),this}clamp(e,i){return this.x=vt(this.x,e.x,i.x),this.y=vt(this.y,e.y,i.y),this}clampScalar(e,i){return this.x=vt(this.x,e,i),this.y=vt(this.y,e,i),this}clampLength(e,i){const s=this.length();return this.divideScalar(s||1).multiplyScalar(vt(s,e,i))}floor(){return this.x=Math.floor(this.x),this.y=Math.floor(this.y),this}ceil(){return this.x=Math.ceil(this.x),this.y=Math.ceil(this.y),this}round(){return this.x=Math.round(this.x),this.y=Math.round(this.y),this}roundToZero(){return this.x=Math.trunc(this.x),this.y=Math.trunc(this.y),this}negate(){return this.x=-this.x,this.y=-this.y,this}dot(e){return this.x*e.x+this.y*e.y}cross(e){return this.x*e.y-this.y*e.x}lengthSq(){return this.x*this.x+this.y*this.y}length(){return Math.sqrt(this.x*this.x+this.y*this.y)}manhattanLength(){return Math.abs(this.x)+Math.abs(this.y)}normalize(){return this.divideScalar(this.length()||1)}angle(){return Math.atan2(-this.y,-this.x)+Math.PI}angleTo(e){const i=Math.sqrt(this.lengthSq()*e.lengthSq());if(i===0)return Math.PI/2;const s=this.dot(e)/i;return Math.acos(vt(s,-1,1))}distanceTo(e){return Math.sqrt(this.distanceToSquared(e))}distanceToSquared(e){const i=this.x-e.x,s=this.y-e.y;return i*i+s*s}manhattanDistanceTo(e){return Math.abs(this.x-e.x)+Math.abs(this.y-e.y)}setLength(e){return this.normalize().multiplyScalar(e)}lerp(e,i){return this.x+=(e.x-this.x)*i,this.y+=(e.y-this.y)*i,this}lerpVectors(e,i,s){return this.x=e.x+(i.x-e.x)*s,this.y=e.y+(i.y-e.y)*s,this}equals(e){return e.x===this.x&&e.y===this.y}fromArray(e,i=0){return this.x=e[i],this.y=e[i+1],this}toArray(e=[],i=0){return e[i]=this.x,e[i+1]=this.y,e}fromBufferAttribute(e,i){return this.x=e.getX(i),this.y=e.getY(i),this}rotateAround(e,i){const s=Math.cos(i),l=Math.sin(i),c=this.x-e.x,d=this.y-e.y;return this.x=c*s-d*l+e.x,this.y=c*l+d*s+e.y,this}random(){return this.x=Math.random(),this.y=Math.random(),this}*[Symbol.iterator](){yield this.x,yield this.y}}class rs{constructor(e=0,i=0,s=0,l=1){this.isQuaternion=!0,this._x=e,this._y=i,this._z=s,this._w=l}static slerpFlat(e,i,s,l,c,d,p){let m=s[l+0],h=s[l+1],v=s[l+2],y=s[l+3],g=c[d+0],x=c[d+1],E=c[d+2],w=c[d+3];if(y!==w||m!==g||h!==x||v!==E){let b=m*g+h*x+v*E+y*w;b<0&&(g=-g,x=-x,E=-E,w=-w,b=-b);let S=1-p;if(b<.9995){const C=Math.acos(b),U=Math.sin(C);S=Math.sin(S*C)/U,p=Math.sin(p*C)/U,m=m*S+g*p,h=h*S+x*p,v=v*S+E*p,y=y*S+w*p}else{m=m*S+g*p,h=h*S+x*p,v=v*S+E*p,y=y*S+w*p;const C=1/Math.sqrt(m*m+h*h+v*v+y*y);m*=C,h*=C,v*=C,y*=C}}e[i]=m,e[i+1]=h,e[i+2]=v,e[i+3]=y}static multiplyQuaternionsFlat(e,i,s,l,c,d){const p=s[l],m=s[l+1],h=s[l+2],v=s[l+3],y=c[d],g=c[d+1],x=c[d+2],E=c[d+3];return e[i]=p*E+v*y+m*x-h*g,e[i+1]=m*E+v*g+h*y-p*x,e[i+2]=h*E+v*x+p*g-m*y,e[i+3]=v*E-p*y-m*g-h*x,e}get x(){return this._x}set x(e){this._x=e,this._onChangeCallback()}get y(){return this._y}set y(e){this._y=e,this._onChangeCallback()}get z(){return this._z}set z(e){this._z=e,this._onChangeCallback()}get w(){return this._w}set w(e){this._w=e,this._onChangeCallback()}set(e,i,s,l){return this._x=e,this._y=i,this._z=s,this._w=l,this._onChangeCallback(),this}clone(){return new this.constructor(this._x,this._y,this._z,this._w)}copy(e){return this._x=e.x,this._y=e.y,this._z=e.z,this._w=e.w,this._onChangeCallback(),this}setFromEuler(e,i=!0){const s=e._x,l=e._y,c=e._z,d=e._order,p=Math.cos,m=Math.sin,h=p(s/2),v=p(l/2),y=p(c/2),g=m(s/2),x=m(l/2),E=m(c/2);switch(d){case"XYZ":this._x=g*v*y+h*x*E,this._y=h*x*y-g*v*E,this._z=h*v*E+g*x*y,this._w=h*v*y-g*x*E;break;case"YXZ":this._x=g*v*y+h*x*E,this._y=h*x*y-g*v*E,this._z=h*v*E-g*x*y,this._w=h*v*y+g*x*E;break;case"ZXY":this._x=g*v*y-h*x*E,this._y=h*x*y+g*v*E,this._z=h*v*E+g*x*y,this._w=h*v*y-g*x*E;break;case"ZYX":this._x=g*v*y-h*x*E,this._y=h*x*y+g*v*E,this._z=h*v*E-g*x*y,this._w=h*v*y+g*x*E;break;case"YZX":this._x=g*v*y+h*x*E,this._y=h*x*y+g*v*E,this._z=h*v*E-g*x*y,this._w=h*v*y-g*x*E;break;case"XZY":this._x=g*v*y-h*x*E,this._y=h*x*y-g*v*E,this._z=h*v*E+g*x*y,this._w=h*v*y+g*x*E;break;default:at("Quaternion: .setFromEuler() encountered an unknown order: "+d)}return i===!0&&this._onChangeCallback(),this}setFromAxisAngle(e,i){const s=i/2,l=Math.sin(s);return this._x=e.x*l,this._y=e.y*l,this._z=e.z*l,this._w=Math.cos(s),this._onChangeCallback(),this}setFromRotationMatrix(e){const i=e.elements,s=i[0],l=i[4],c=i[8],d=i[1],p=i[5],m=i[9],h=i[2],v=i[6],y=i[10],g=s+p+y;if(g>0){const x=.5/Math.sqrt(g+1);this._w=.25/x,this._x=(v-m)*x,this._y=(c-h)*x,this._z=(d-l)*x}else if(s>p&&s>y){const x=2*Math.sqrt(1+s-p-y);this._w=(v-m)/x,this._x=.25*x,this._y=(l+d)/x,this._z=(c+h)/x}else if(p>y){const x=2*Math.sqrt(1+p-s-y);this._w=(c-h)/x,this._x=(l+d)/x,this._y=.25*x,this._z=(m+v)/x}else{const x=2*Math.sqrt(1+y-s-p);this._w=(d-l)/x,this._x=(c+h)/x,this._y=(m+v)/x,this._z=.25*x}return this._onChangeCallback(),this}setFromUnitVectors(e,i){let s=e.dot(i)+1;return s<1e-8?(s=0,Math.abs(e.x)>Math.abs(e.z)?(this._x=-e.y,this._y=e.x,this._z=0,this._w=s):(this._x=0,this._y=-e.z,this._z=e.y,this._w=s)):(this._x=e.y*i.z-e.z*i.y,this._y=e.z*i.x-e.x*i.z,this._z=e.x*i.y-e.y*i.x,this._w=s),this.normalize()}angleTo(e){return 2*Math.acos(Math.abs(vt(this.dot(e),-1,1)))}rotateTowards(e,i){const s=this.angleTo(e);if(s===0)return this;const l=Math.min(1,i/s);return this.slerp(e,l),this}identity(){return this.set(0,0,0,1)}invert(){return this.conjugate()}conjugate(){return this._x*=-1,this._y*=-1,this._z*=-1,this._onChangeCallback(),this}dot(e){return this._x*e._x+this._y*e._y+this._z*e._z+this._w*e._w}lengthSq(){return this._x*this._x+this._y*this._y+this._z*this._z+this._w*this._w}length(){return Math.sqrt(this._x*this._x+this._y*this._y+this._z*this._z+this._w*this._w)}normalize(){let e=this.length();return e===0?(this._x=0,this._y=0,this._z=0,this._w=1):(e=1/e,this._x=this._x*e,this._y=this._y*e,this._z=this._z*e,this._w=this._w*e),this._onChangeCallback(),this}multiply(e){return this.multiplyQuaternions(this,e)}premultiply(e){return this.multiplyQuaternions(e,this)}multiplyQuaternions(e,i){const s=e._x,l=e._y,c=e._z,d=e._w,p=i._x,m=i._y,h=i._z,v=i._w;return this._x=s*v+d*p+l*h-c*m,this._y=l*v+d*m+c*p-s*h,this._z=c*v+d*h+s*m-l*p,this._w=d*v-s*p-l*m-c*h,this._onChangeCallback(),this}slerp(e,i){let s=e._x,l=e._y,c=e._z,d=e._w,p=this.dot(e);p<0&&(s=-s,l=-l,c=-c,d=-d,p=-p);let m=1-i;if(p<.9995){const h=Math.acos(p),v=Math.sin(h);m=Math.sin(m*h)/v,i=Math.sin(i*h)/v,this._x=this._x*m+s*i,this._y=this._y*m+l*i,this._z=this._z*m+c*i,this._w=this._w*m+d*i,this._onChangeCallback()}else this._x=this._x*m+s*i,this._y=this._y*m+l*i,this._z=this._z*m+c*i,this._w=this._w*m+d*i,this.normalize();return this}slerpQuaternions(e,i,s){return this.copy(e).slerp(i,s)}random(){const e=2*Math.PI*Math.random(),i=2*Math.PI*Math.random(),s=Math.random(),l=Math.sqrt(1-s),c=Math.sqrt(s);return this.set(l*Math.sin(e),l*Math.cos(e),c*Math.sin(i),c*Math.cos(i))}equals(e){return e._x===this._x&&e._y===this._y&&e._z===this._z&&e._w===this._w}fromArray(e,i=0){return this._x=e[i],this._y=e[i+1],this._z=e[i+2],this._w=e[i+3],this._onChangeCallback(),this}toArray(e=[],i=0){return e[i]=this._x,e[i+1]=this._y,e[i+2]=this._z,e[i+3]=this._w,e}fromBufferAttribute(e,i){return this._x=e.getX(i),this._y=e.getY(i),this._z=e.getZ(i),this._w=e.getW(i),this._onChangeCallback(),this}toJSON(){return this.toArray()}_onChange(e){return this._onChangeCallback=e,this}_onChangeCallback(){}*[Symbol.iterator](){yield this._x,yield this._y,yield this._z,yield this._w}}class K{constructor(e=0,i=0,s=0){K.prototype.isVector3=!0,this.x=e,this.y=i,this.z=s}set(e,i,s){return s===void 0&&(s=this.z),this.x=e,this.y=i,this.z=s,this}setScalar(e){return this.x=e,this.y=e,this.z=e,this}setX(e){return this.x=e,this}setY(e){return this.y=e,this}setZ(e){return this.z=e,this}setComponent(e,i){switch(e){case 0:this.x=i;break;case 1:this.y=i;break;case 2:this.z=i;break;default:throw new Error("index is out of range: "+e)}return this}getComponent(e){switch(e){case 0:return this.x;case 1:return this.y;case 2:return this.z;default:throw new Error("index is out of range: "+e)}}clone(){return new this.constructor(this.x,this.y,this.z)}copy(e){return this.x=e.x,this.y=e.y,this.z=e.z,this}add(e){return this.x+=e.x,this.y+=e.y,this.z+=e.z,this}addScalar(e){return this.x+=e,this.y+=e,this.z+=e,this}addVectors(e,i){return this.x=e.x+i.x,this.y=e.y+i.y,this.z=e.z+i.z,this}addScaledVector(e,i){return this.x+=e.x*i,this.y+=e.y*i,this.z+=e.z*i,this}sub(e){return this.x-=e.x,this.y-=e.y,this.z-=e.z,this}subScalar(e){return this.x-=e,this.y-=e,this.z-=e,this}subVectors(e,i){return this.x=e.x-i.x,this.y=e.y-i.y,this.z=e.z-i.z,this}multiply(e){return this.x*=e.x,this.y*=e.y,this.z*=e.z,this}multiplyScalar(e){return this.x*=e,this.y*=e,this.z*=e,this}multiplyVectors(e,i){return this.x=e.x*i.x,this.y=e.y*i.y,this.z=e.z*i.z,this}applyEuler(e){return this.applyQuaternion(e_.setFromEuler(e))}applyAxisAngle(e,i){return this.applyQuaternion(e_.setFromAxisAngle(e,i))}applyMatrix3(e){const i=this.x,s=this.y,l=this.z,c=e.elements;return this.x=c[0]*i+c[3]*s+c[6]*l,this.y=c[1]*i+c[4]*s+c[7]*l,this.z=c[2]*i+c[5]*s+c[8]*l,this}applyNormalMatrix(e){return this.applyMatrix3(e).normalize()}applyMatrix4(e){const i=this.x,s=this.y,l=this.z,c=e.elements,d=1/(c[3]*i+c[7]*s+c[11]*l+c[15]);return this.x=(c[0]*i+c[4]*s+c[8]*l+c[12])*d,this.y=(c[1]*i+c[5]*s+c[9]*l+c[13])*d,this.z=(c[2]*i+c[6]*s+c[10]*l+c[14])*d,this}applyQuaternion(e){const i=this.x,s=this.y,l=this.z,c=e.x,d=e.y,p=e.z,m=e.w,h=2*(d*l-p*s),v=2*(p*i-c*l),y=2*(c*s-d*i);return this.x=i+m*h+d*y-p*v,this.y=s+m*v+p*h-c*y,this.z=l+m*y+c*v-d*h,this}project(e){return this.applyMatrix4(e.matrixWorldInverse).applyMatrix4(e.projectionMatrix)}unproject(e){return this.applyMatrix4(e.projectionMatrixInverse).applyMatrix4(e.matrixWorld)}transformDirection(e){const i=this.x,s=this.y,l=this.z,c=e.elements;return this.x=c[0]*i+c[4]*s+c[8]*l,this.y=c[1]*i+c[5]*s+c[9]*l,this.z=c[2]*i+c[6]*s+c[10]*l,this.normalize()}divide(e){return this.x/=e.x,this.y/=e.y,this.z/=e.z,this}divideScalar(e){return this.multiplyScalar(1/e)}min(e){return this.x=Math.min(this.x,e.x),this.y=Math.min(this.y,e.y),this.z=Math.min(this.z,e.z),this}max(e){return this.x=Math.max(this.x,e.x),this.y=Math.max(this.y,e.y),this.z=Math.max(this.z,e.z),this}clamp(e,i){return this.x=vt(this.x,e.x,i.x),this.y=vt(this.y,e.y,i.y),this.z=vt(this.z,e.z,i.z),this}clampScalar(e,i){return this.x=vt(this.x,e,i),this.y=vt(this.y,e,i),this.z=vt(this.z,e,i),this}clampLength(e,i){const s=this.length();return this.divideScalar(s||1).multiplyScalar(vt(s,e,i))}floor(){return this.x=Math.floor(this.x),this.y=Math.floor(this.y),this.z=Math.floor(this.z),this}ceil(){return this.x=Math.ceil(this.x),this.y=Math.ceil(this.y),this.z=Math.ceil(this.z),this}round(){return this.x=Math.round(this.x),this.y=Math.round(this.y),this.z=Math.round(this.z),this}roundToZero(){return this.x=Math.trunc(this.x),this.y=Math.trunc(this.y),this.z=Math.trunc(this.z),this}negate(){return this.x=-this.x,this.y=-this.y,this.z=-this.z,this}dot(e){return this.x*e.x+this.y*e.y+this.z*e.z}lengthSq(){return this.x*this.x+this.y*this.y+this.z*this.z}length(){return Math.sqrt(this.x*this.x+this.y*this.y+this.z*this.z)}manhattanLength(){return Math.abs(this.x)+Math.abs(this.y)+Math.abs(this.z)}normalize(){return this.divideScalar(this.length()||1)}setLength(e){return this.normalize().multiplyScalar(e)}lerp(e,i){return this.x+=(e.x-this.x)*i,this.y+=(e.y-this.y)*i,this.z+=(e.z-this.z)*i,this}lerpVectors(e,i,s){return this.x=e.x+(i.x-e.x)*s,this.y=e.y+(i.y-e.y)*s,this.z=e.z+(i.z-e.z)*s,this}cross(e){return this.crossVectors(this,e)}crossVectors(e,i){const s=e.x,l=e.y,c=e.z,d=i.x,p=i.y,m=i.z;return this.x=l*m-c*p,this.y=c*d-s*m,this.z=s*p-l*d,this}projectOnVector(e){const i=e.lengthSq();if(i===0)return this.set(0,0,0);const s=e.dot(this)/i;return this.copy(e).multiplyScalar(s)}projectOnPlane(e){return _d.copy(this).projectOnVector(e),this.sub(_d)}reflect(e){return this.sub(_d.copy(e).multiplyScalar(2*this.dot(e)))}angleTo(e){const i=Math.sqrt(this.lengthSq()*e.lengthSq());if(i===0)return Math.PI/2;const s=this.dot(e)/i;return Math.acos(vt(s,-1,1))}distanceTo(e){return Math.sqrt(this.distanceToSquared(e))}distanceToSquared(e){const i=this.x-e.x,s=this.y-e.y,l=this.z-e.z;return i*i+s*s+l*l}manhattanDistanceTo(e){return Math.abs(this.x-e.x)+Math.abs(this.y-e.y)+Math.abs(this.z-e.z)}setFromSpherical(e){return this.setFromSphericalCoords(e.radius,e.phi,e.theta)}setFromSphericalCoords(e,i,s){const l=Math.sin(i)*e;return this.x=l*Math.sin(s),this.y=Math.cos(i)*e,this.z=l*Math.cos(s),this}setFromCylindrical(e){return this.setFromCylindricalCoords(e.radius,e.theta,e.y)}setFromCylindricalCoords(e,i,s){return this.x=e*Math.sin(i),this.y=s,this.z=e*Math.cos(i),this}setFromMatrixPosition(e){const i=e.elements;return this.x=i[12],this.y=i[13],this.z=i[14],this}setFromMatrixScale(e){const i=this.setFromMatrixColumn(e,0).length(),s=this.setFromMatrixColumn(e,1).length(),l=this.setFromMatrixColumn(e,2).length();return this.x=i,this.y=s,this.z=l,this}setFromMatrixColumn(e,i){return this.fromArray(e.elements,i*4)}setFromMatrix3Column(e,i){return this.fromArray(e.elements,i*3)}setFromEuler(e){return this.x=e._x,this.y=e._y,this.z=e._z,this}setFromColor(e){return this.x=e.r,this.y=e.g,this.z=e.b,this}equals(e){return e.x===this.x&&e.y===this.y&&e.z===this.z}fromArray(e,i=0){return this.x=e[i],this.y=e[i+1],this.z=e[i+2],this}toArray(e=[],i=0){return e[i]=this.x,e[i+1]=this.y,e[i+2]=this.z,e}fromBufferAttribute(e,i){return this.x=e.getX(i),this.y=e.getY(i),this.z=e.getZ(i),this}random(){return this.x=Math.random(),this.y=Math.random(),this.z=Math.random(),this}randomDirection(){const e=Math.random()*Math.PI*2,i=Math.random()*2-1,s=Math.sqrt(1-i*i);return this.x=s*Math.cos(e),this.y=i,this.z=s*Math.sin(e),this}*[Symbol.iterator](){yield this.x,yield this.y,yield this.z}}const _d=new K,e_=new rs;class ht{constructor(e,i,s,l,c,d,p,m,h){ht.prototype.isMatrix3=!0,this.elements=[1,0,0,0,1,0,0,0,1],e!==void 0&&this.set(e,i,s,l,c,d,p,m,h)}set(e,i,s,l,c,d,p,m,h){const v=this.elements;return v[0]=e,v[1]=l,v[2]=p,v[3]=i,v[4]=c,v[5]=m,v[6]=s,v[7]=d,v[8]=h,this}identity(){return this.set(1,0,0,0,1,0,0,0,1),this}copy(e){const i=this.elements,s=e.elements;return i[0]=s[0],i[1]=s[1],i[2]=s[2],i[3]=s[3],i[4]=s[4],i[5]=s[5],i[6]=s[6],i[7]=s[7],i[8]=s[8],this}extractBasis(e,i,s){return e.setFromMatrix3Column(this,0),i.setFromMatrix3Column(this,1),s.setFromMatrix3Column(this,2),this}setFromMatrix4(e){const i=e.elements;return this.set(i[0],i[4],i[8],i[1],i[5],i[9],i[2],i[6],i[10]),this}multiply(e){return this.multiplyMatrices(this,e)}premultiply(e){return this.multiplyMatrices(e,this)}multiplyMatrices(e,i){const s=e.elements,l=i.elements,c=this.elements,d=s[0],p=s[3],m=s[6],h=s[1],v=s[4],y=s[7],g=s[2],x=s[5],E=s[8],w=l[0],b=l[3],S=l[6],C=l[1],U=l[4],N=l[7],V=l[2],H=l[5],F=l[8];return c[0]=d*w+p*C+m*V,c[3]=d*b+p*U+m*H,c[6]=d*S+p*N+m*F,c[1]=h*w+v*C+y*V,c[4]=h*b+v*U+y*H,c[7]=h*S+v*N+y*F,c[2]=g*w+x*C+E*V,c[5]=g*b+x*U+E*H,c[8]=g*S+x*N+E*F,this}multiplyScalar(e){const i=this.elements;return i[0]*=e,i[3]*=e,i[6]*=e,i[1]*=e,i[4]*=e,i[7]*=e,i[2]*=e,i[5]*=e,i[8]*=e,this}determinant(){const e=this.elements,i=e[0],s=e[1],l=e[2],c=e[3],d=e[4],p=e[5],m=e[6],h=e[7],v=e[8];return i*d*v-i*p*h-s*c*v+s*p*m+l*c*h-l*d*m}invert(){const e=this.elements,i=e[0],s=e[1],l=e[2],c=e[3],d=e[4],p=e[5],m=e[6],h=e[7],v=e[8],y=v*d-p*h,g=p*m-v*c,x=h*c-d*m,E=i*y+s*g+l*x;if(E===0)return this.set(0,0,0,0,0,0,0,0,0);const w=1/E;return e[0]=y*w,e[1]=(l*h-v*s)*w,e[2]=(p*s-l*d)*w,e[3]=g*w,e[4]=(v*i-l*m)*w,e[5]=(l*c-p*i)*w,e[6]=x*w,e[7]=(s*m-h*i)*w,e[8]=(d*i-s*c)*w,this}transpose(){let e;const i=this.elements;return e=i[1],i[1]=i[3],i[3]=e,e=i[2],i[2]=i[6],i[6]=e,e=i[5],i[5]=i[7],i[7]=e,this}getNormalMatrix(e){return this.setFromMatrix4(e).invert().transpose()}transposeIntoArray(e){const i=this.elements;return e[0]=i[0],e[1]=i[3],e[2]=i[6],e[3]=i[1],e[4]=i[4],e[5]=i[7],e[6]=i[2],e[7]=i[5],e[8]=i[8],this}setUvTransform(e,i,s,l,c,d,p){const m=Math.cos(c),h=Math.sin(c);return this.set(s*m,s*h,-s*(m*d+h*p)+d+e,-l*h,l*m,-l*(-h*d+m*p)+p+i,0,0,1),this}scale(e,i){return this.premultiply(vd.makeScale(e,i)),this}rotate(e){return this.premultiply(vd.makeRotation(-e)),this}translate(e,i){return this.premultiply(vd.makeTranslation(e,i)),this}makeTranslation(e,i){return e.isVector2?this.set(1,0,e.x,0,1,e.y,0,0,1):this.set(1,0,e,0,1,i,0,0,1),this}makeRotation(e){const i=Math.cos(e),s=Math.sin(e);return this.set(i,-s,0,s,i,0,0,0,1),this}makeScale(e,i){return this.set(e,0,0,0,i,0,0,0,1),this}equals(e){const i=this.elements,s=e.elements;for(let l=0;l<9;l++)if(i[l]!==s[l])return!1;return!0}fromArray(e,i=0){for(let s=0;s<9;s++)this.elements[s]=e[s+i];return this}toArray(e=[],i=0){const s=this.elements;return e[i]=s[0],e[i+1]=s[1],e[i+2]=s[2],e[i+3]=s[3],e[i+4]=s[4],e[i+5]=s[5],e[i+6]=s[6],e[i+7]=s[7],e[i+8]=s[8],e}clone(){return new this.constructor().fromArray(this.elements)}}const vd=new ht,t_=new ht().set(.4123908,.3575843,.1804808,.212639,.7151687,.0721923,.0193308,.1191948,.9505322),n_=new ht().set(3.2409699,-1.5373832,-.4986108,-.9692436,1.8759675,.0415551,.0556301,-.203977,1.0569715);function IS(){const o={enabled:!0,workingColorSpace:Vr,spaces:{},convert:function(l,c,d){return this.enabled===!1||c===d||!c||!d||(this.spaces[c].transfer===zt&&(l.r=Sa(l.r),l.g=Sa(l.g),l.b=Sa(l.b)),this.spaces[c].primaries!==this.spaces[d].primaries&&(l.applyMatrix3(this.spaces[c].toXYZ),l.applyMatrix3(this.spaces[d].fromXYZ)),this.spaces[d].transfer===zt&&(l.r=zr(l.r),l.g=zr(l.g),l.b=zr(l.b))),l},workingToColorSpace:function(l,c){return this.convert(l,this.workingColorSpace,c)},colorSpaceToWorking:function(l,c){return this.convert(l,c,this.workingColorSpace)},getPrimaries:function(l){return this.spaces[l].primaries},getTransfer:function(l){return l===is?Yc:this.spaces[l].transfer},getToneMappingMode:function(l){return this.spaces[l].outputColorSpaceConfig.toneMappingMode||"standard"},getLuminanceCoefficients:function(l,c=this.workingColorSpace){return l.fromArray(this.spaces[c].luminanceCoefficients)},define:function(l){Object.assign(this.spaces,l)},_getMatrix:function(l,c,d){return l.copy(this.spaces[c].toXYZ).multiply(this.spaces[d].fromXYZ)},_getDrawingBufferColorSpace:function(l){return this.spaces[l].outputColorSpaceConfig.drawingBufferColorSpace},_getUnpackColorSpace:function(l=this.workingColorSpace){return this.spaces[l].workingColorSpaceConfig.unpackColorSpace},fromWorkingColorSpace:function(l,c){return Kc("ColorManagement: .fromWorkingColorSpace() has been renamed to .workingToColorSpace()."),o.workingToColorSpace(l,c)},toWorkingColorSpace:function(l,c){return Kc("ColorManagement: .toWorkingColorSpace() has been renamed to .colorSpaceToWorking()."),o.colorSpaceToWorking(l,c)}},e=[.64,.33,.3,.6,.15,.06],i=[.2126,.7152,.0722],s=[.3127,.329];return o.define({[Vr]:{primaries:e,whitePoint:s,transfer:Yc,toXYZ:t_,fromXYZ:n_,luminanceCoefficients:i,workingColorSpaceConfig:{unpackColorSpace:gi},outputColorSpaceConfig:{drawingBufferColorSpace:gi}},[gi]:{primaries:e,whitePoint:s,transfer:zt,toXYZ:t_,fromXYZ:n_,luminanceCoefficients:i,outputColorSpaceConfig:{drawingBufferColorSpace:gi}}}),o}const Tt=IS();function Sa(o){return o<.04045?o*.0773993808:Math.pow(o*.9478672986+.0521327014,2.4)}function zr(o){return o<.0031308?o*12.92:1.055*Math.pow(o,.41666)-.055}let Sr;class FS{static getDataURL(e,i="image/png"){if(/^data:/i.test(e.src)||typeof HTMLCanvasElement>"u")return e.src;let s;if(e instanceof HTMLCanvasElement)s=e;else{Sr===void 0&&(Sr=Zc("canvas")),Sr.width=e.width,Sr.height=e.height;const l=Sr.getContext("2d");e instanceof ImageData?l.putImageData(e,0,0):l.drawImage(e,0,0,e.width,e.height),s=Sr}return s.toDataURL(i)}static sRGBToLinear(e){if(typeof HTMLImageElement<"u"&&e instanceof HTMLImageElement||typeof HTMLCanvasElement<"u"&&e instanceof HTMLCanvasElement||typeof ImageBitmap<"u"&&e instanceof ImageBitmap){const i=Zc("canvas");i.width=e.width,i.height=e.height;const s=i.getContext("2d");s.drawImage(e,0,0,e.width,e.height);const l=s.getImageData(0,0,e.width,e.height),c=l.data;for(let d=0;d<c.length;d++)c[d]=Sa(c[d]/255)*255;return s.putImageData(l,0,0),i}else if(e.data){const i=e.data.slice(0);for(let s=0;s<i.length;s++)i instanceof Uint8Array||i instanceof Uint8ClampedArray?i[s]=Math.floor(Sa(i[s]/255)*255):i[s]=Sa(i[s]);return{data:i,width:e.width,height:e.height}}else return at("ImageUtils.sRGBToLinear(): Unsupported image type. No color space conversion applied."),e}}let zS=0;class Jh{constructor(e=null){this.isSource=!0,Object.defineProperty(this,"id",{value:zS++}),this.uuid=Qo(),this.data=e,this.dataReady=!0,this.version=0}getSize(e){const i=this.data;return typeof HTMLVideoElement<"u"&&i instanceof HTMLVideoElement?e.set(i.videoWidth,i.videoHeight,0):typeof VideoFrame<"u"&&i instanceof VideoFrame?e.set(i.displayHeight,i.displayWidth,0):i!==null?e.set(i.width,i.height,i.depth||0):e.set(0,0,0),e}set needsUpdate(e){e===!0&&this.version++}toJSON(e){const i=e===void 0||typeof e=="string";if(!i&&e.images[this.uuid]!==void 0)return e.images[this.uuid];const s={uuid:this.uuid,url:""},l=this.data;if(l!==null){let c;if(Array.isArray(l)){c=[];for(let d=0,p=l.length;d<p;d++)l[d].isDataTexture?c.push(xd(l[d].image)):c.push(xd(l[d]))}else c=xd(l);s.url=c}return i||(e.images[this.uuid]=s),s}}function xd(o){return typeof HTMLImageElement<"u"&&o instanceof HTMLImageElement||typeof HTMLCanvasElement<"u"&&o instanceof HTMLCanvasElement||typeof ImageBitmap<"u"&&o instanceof ImageBitmap?FS.getDataURL(o):o.data?{data:Array.from(o.data),width:o.width,height:o.height,type:o.data.constructor.name}:(at("Texture: Unable to serialize Texture."),{})}let BS=0;const yd=new K;class Fn extends Fs{constructor(e=Fn.DEFAULT_IMAGE,i=Fn.DEFAULT_MAPPING,s=xa,l=xa,c=Dn,d=Os,p=Di,m=ri,h=Fn.DEFAULT_ANISOTROPY,v=is){super(),this.isTexture=!0,Object.defineProperty(this,"id",{value:BS++}),this.uuid=Qo(),this.name="",this.source=new Jh(e),this.mipmaps=[],this.mapping=i,this.channel=0,this.wrapS=s,this.wrapT=l,this.magFilter=c,this.minFilter=d,this.anisotropy=h,this.format=p,this.internalFormat=null,this.type=m,this.offset=new ct(0,0),this.repeat=new ct(1,1),this.center=new ct(0,0),this.rotation=0,this.matrixAutoUpdate=!0,this.matrix=new ht,this.generateMipmaps=!0,this.premultiplyAlpha=!1,this.flipY=!0,this.unpackAlignment=4,this.colorSpace=v,this.userData={},this.updateRanges=[],this.version=0,this.onUpdate=null,this.renderTarget=null,this.isRenderTargetTexture=!1,this.isArrayTexture=!!(e&&e.depth&&e.depth>1),this.pmremVersion=0}get width(){return this.source.getSize(yd).x}get height(){return this.source.getSize(yd).y}get depth(){return this.source.getSize(yd).z}get image(){return this.source.data}set image(e=null){this.source.data=e}updateMatrix(){this.matrix.setUvTransform(this.offset.x,this.offset.y,this.repeat.x,this.repeat.y,this.rotation,this.center.x,this.center.y)}addUpdateRange(e,i){this.updateRanges.push({start:e,count:i})}clearUpdateRanges(){this.updateRanges.length=0}clone(){return new this.constructor().copy(this)}copy(e){return this.name=e.name,this.source=e.source,this.mipmaps=e.mipmaps.slice(0),this.mapping=e.mapping,this.channel=e.channel,this.wrapS=e.wrapS,this.wrapT=e.wrapT,this.magFilter=e.magFilter,this.minFilter=e.minFilter,this.anisotropy=e.anisotropy,this.format=e.format,this.internalFormat=e.internalFormat,this.type=e.type,this.offset.copy(e.offset),this.repeat.copy(e.repeat),this.center.copy(e.center),this.rotation=e.rotation,this.matrixAutoUpdate=e.matrixAutoUpdate,this.matrix.copy(e.matrix),this.generateMipmaps=e.generateMipmaps,this.premultiplyAlpha=e.premultiplyAlpha,this.flipY=e.flipY,this.unpackAlignment=e.unpackAlignment,this.colorSpace=e.colorSpace,this.renderTarget=e.renderTarget,this.isRenderTargetTexture=e.isRenderTargetTexture,this.isArrayTexture=e.isArrayTexture,this.userData=JSON.parse(JSON.stringify(e.userData)),this.needsUpdate=!0,this}setValues(e){for(const i in e){const s=e[i];if(s===void 0){at(`Texture.setValues(): parameter '${i}' has value of undefined.`);continue}const l=this[i];if(l===void 0){at(`Texture.setValues(): property '${i}' does not exist.`);continue}l&&s&&l.isVector2&&s.isVector2||l&&s&&l.isVector3&&s.isVector3||l&&s&&l.isMatrix3&&s.isMatrix3?l.copy(s):this[i]=s}}toJSON(e){const i=e===void 0||typeof e=="string";if(!i&&e.textures[this.uuid]!==void 0)return e.textures[this.uuid];const s={metadata:{version:4.7,type:"Texture",generator:"Texture.toJSON"},uuid:this.uuid,name:this.name,image:this.source.toJSON(e).uuid,mapping:this.mapping,channel:this.channel,repeat:[this.repeat.x,this.repeat.y],offset:[this.offset.x,this.offset.y],center:[this.center.x,this.center.y],rotation:this.rotation,wrap:[this.wrapS,this.wrapT],format:this.format,internalFormat:this.internalFormat,type:this.type,colorSpace:this.colorSpace,minFilter:this.minFilter,magFilter:this.magFilter,anisotropy:this.anisotropy,flipY:this.flipY,generateMipmaps:this.generateMipmaps,premultiplyAlpha:this.premultiplyAlpha,unpackAlignment:this.unpackAlignment};return Object.keys(this.userData).length>0&&(s.userData=this.userData),i||(e.textures[this.uuid]=s),s}dispose(){this.dispatchEvent({type:"dispose"})}transformUv(e){if(this.mapping!==sv)return e;if(e.applyMatrix3(this.matrix),e.x<0||e.x>1)switch(this.wrapS){case ih:e.x=e.x-Math.floor(e.x);break;case xa:e.x=e.x<0?0:1;break;case ah:Math.abs(Math.floor(e.x)%2)===1?e.x=Math.ceil(e.x)-e.x:e.x=e.x-Math.floor(e.x);break}if(e.y<0||e.y>1)switch(this.wrapT){case ih:e.y=e.y-Math.floor(e.y);break;case xa:e.y=e.y<0?0:1;break;case ah:Math.abs(Math.floor(e.y)%2)===1?e.y=Math.ceil(e.y)-e.y:e.y=e.y-Math.floor(e.y);break}return this.flipY&&(e.y=1-e.y),e}set needsUpdate(e){e===!0&&(this.version++,this.source.needsUpdate=!0)}set needsPMREMUpdate(e){e===!0&&this.pmremVersion++}}Fn.DEFAULT_IMAGE=null;Fn.DEFAULT_MAPPING=sv;Fn.DEFAULT_ANISOTROPY=1;class nn{constructor(e=0,i=0,s=0,l=1){nn.prototype.isVector4=!0,this.x=e,this.y=i,this.z=s,this.w=l}get width(){return this.z}set width(e){this.z=e}get height(){return this.w}set height(e){this.w=e}set(e,i,s,l){return this.x=e,this.y=i,this.z=s,this.w=l,this}setScalar(e){return this.x=e,this.y=e,this.z=e,this.w=e,this}setX(e){return this.x=e,this}setY(e){return this.y=e,this}setZ(e){return this.z=e,this}setW(e){return this.w=e,this}setComponent(e,i){switch(e){case 0:this.x=i;break;case 1:this.y=i;break;case 2:this.z=i;break;case 3:this.w=i;break;default:throw new Error("index is out of range: "+e)}return this}getComponent(e){switch(e){case 0:return this.x;case 1:return this.y;case 2:return this.z;case 3:return this.w;default:throw new Error("index is out of range: "+e)}}clone(){return new this.constructor(this.x,this.y,this.z,this.w)}copy(e){return this.x=e.x,this.y=e.y,this.z=e.z,this.w=e.w!==void 0?e.w:1,this}add(e){return this.x+=e.x,this.y+=e.y,this.z+=e.z,this.w+=e.w,this}addScalar(e){return this.x+=e,this.y+=e,this.z+=e,this.w+=e,this}addVectors(e,i){return this.x=e.x+i.x,this.y=e.y+i.y,this.z=e.z+i.z,this.w=e.w+i.w,this}addScaledVector(e,i){return this.x+=e.x*i,this.y+=e.y*i,this.z+=e.z*i,this.w+=e.w*i,this}sub(e){return this.x-=e.x,this.y-=e.y,this.z-=e.z,this.w-=e.w,this}subScalar(e){return this.x-=e,this.y-=e,this.z-=e,this.w-=e,this}subVectors(e,i){return this.x=e.x-i.x,this.y=e.y-i.y,this.z=e.z-i.z,this.w=e.w-i.w,this}multiply(e){return this.x*=e.x,this.y*=e.y,this.z*=e.z,this.w*=e.w,this}multiplyScalar(e){return this.x*=e,this.y*=e,this.z*=e,this.w*=e,this}applyMatrix4(e){const i=this.x,s=this.y,l=this.z,c=this.w,d=e.elements;return this.x=d[0]*i+d[4]*s+d[8]*l+d[12]*c,this.y=d[1]*i+d[5]*s+d[9]*l+d[13]*c,this.z=d[2]*i+d[6]*s+d[10]*l+d[14]*c,this.w=d[3]*i+d[7]*s+d[11]*l+d[15]*c,this}divide(e){return this.x/=e.x,this.y/=e.y,this.z/=e.z,this.w/=e.w,this}divideScalar(e){return this.multiplyScalar(1/e)}setAxisAngleFromQuaternion(e){this.w=2*Math.acos(e.w);const i=Math.sqrt(1-e.w*e.w);return i<1e-4?(this.x=1,this.y=0,this.z=0):(this.x=e.x/i,this.y=e.y/i,this.z=e.z/i),this}setAxisAngleFromRotationMatrix(e){let i,s,l,c;const m=e.elements,h=m[0],v=m[4],y=m[8],g=m[1],x=m[5],E=m[9],w=m[2],b=m[6],S=m[10];if(Math.abs(v-g)<.01&&Math.abs(y-w)<.01&&Math.abs(E-b)<.01){if(Math.abs(v+g)<.1&&Math.abs(y+w)<.1&&Math.abs(E+b)<.1&&Math.abs(h+x+S-3)<.1)return this.set(1,0,0,0),this;i=Math.PI;const U=(h+1)/2,N=(x+1)/2,V=(S+1)/2,H=(v+g)/4,F=(y+w)/4,T=(E+b)/4;return U>N&&U>V?U<.01?(s=0,l=.707106781,c=.707106781):(s=Math.sqrt(U),l=H/s,c=F/s):N>V?N<.01?(s=.707106781,l=0,c=.707106781):(l=Math.sqrt(N),s=H/l,c=T/l):V<.01?(s=.707106781,l=.707106781,c=0):(c=Math.sqrt(V),s=F/c,l=T/c),this.set(s,l,c,i),this}let C=Math.sqrt((b-E)*(b-E)+(y-w)*(y-w)+(g-v)*(g-v));return Math.abs(C)<.001&&(C=1),this.x=(b-E)/C,this.y=(y-w)/C,this.z=(g-v)/C,this.w=Math.acos((h+x+S-1)/2),this}setFromMatrixPosition(e){const i=e.elements;return this.x=i[12],this.y=i[13],this.z=i[14],this.w=i[15],this}min(e){return this.x=Math.min(this.x,e.x),this.y=Math.min(this.y,e.y),this.z=Math.min(this.z,e.z),this.w=Math.min(this.w,e.w),this}max(e){return this.x=Math.max(this.x,e.x),this.y=Math.max(this.y,e.y),this.z=Math.max(this.z,e.z),this.w=Math.max(this.w,e.w),this}clamp(e,i){return this.x=vt(this.x,e.x,i.x),this.y=vt(this.y,e.y,i.y),this.z=vt(this.z,e.z,i.z),this.w=vt(this.w,e.w,i.w),this}clampScalar(e,i){return this.x=vt(this.x,e,i),this.y=vt(this.y,e,i),this.z=vt(this.z,e,i),this.w=vt(this.w,e,i),this}clampLength(e,i){const s=this.length();return this.divideScalar(s||1).multiplyScalar(vt(s,e,i))}floor(){return this.x=Math.floor(this.x),this.y=Math.floor(this.y),this.z=Math.floor(this.z),this.w=Math.floor(this.w),this}ceil(){return this.x=Math.ceil(this.x),this.y=Math.ceil(this.y),this.z=Math.ceil(this.z),this.w=Math.ceil(this.w),this}round(){return this.x=Math.round(this.x),this.y=Math.round(this.y),this.z=Math.round(this.z),this.w=Math.round(this.w),this}roundToZero(){return this.x=Math.trunc(this.x),this.y=Math.trunc(this.y),this.z=Math.trunc(this.z),this.w=Math.trunc(this.w),this}negate(){return this.x=-this.x,this.y=-this.y,this.z=-this.z,this.w=-this.w,this}dot(e){return this.x*e.x+this.y*e.y+this.z*e.z+this.w*e.w}lengthSq(){return this.x*this.x+this.y*this.y+this.z*this.z+this.w*this.w}length(){return Math.sqrt(this.x*this.x+this.y*this.y+this.z*this.z+this.w*this.w)}manhattanLength(){return Math.abs(this.x)+Math.abs(this.y)+Math.abs(this.z)+Math.abs(this.w)}normalize(){return this.divideScalar(this.length()||1)}setLength(e){return this.normalize().multiplyScalar(e)}lerp(e,i){return this.x+=(e.x-this.x)*i,this.y+=(e.y-this.y)*i,this.z+=(e.z-this.z)*i,this.w+=(e.w-this.w)*i,this}lerpVectors(e,i,s){return this.x=e.x+(i.x-e.x)*s,this.y=e.y+(i.y-e.y)*s,this.z=e.z+(i.z-e.z)*s,this.w=e.w+(i.w-e.w)*s,this}equals(e){return e.x===this.x&&e.y===this.y&&e.z===this.z&&e.w===this.w}fromArray(e,i=0){return this.x=e[i],this.y=e[i+1],this.z=e[i+2],this.w=e[i+3],this}toArray(e=[],i=0){return e[i]=this.x,e[i+1]=this.y,e[i+2]=this.z,e[i+3]=this.w,e}fromBufferAttribute(e,i){return this.x=e.getX(i),this.y=e.getY(i),this.z=e.getZ(i),this.w=e.getW(i),this}random(){return this.x=Math.random(),this.y=Math.random(),this.z=Math.random(),this.w=Math.random(),this}*[Symbol.iterator](){yield this.x,yield this.y,yield this.z,yield this.w}}class HS extends Fs{constructor(e=1,i=1,s={}){super(),s=Object.assign({generateMipmaps:!1,internalFormat:null,minFilter:Dn,depthBuffer:!0,stencilBuffer:!1,resolveDepthBuffer:!0,resolveStencilBuffer:!0,depthTexture:null,samples:0,count:1,depth:1,multiview:!1},s),this.isRenderTarget=!0,this.width=e,this.height=i,this.depth=s.depth,this.scissor=new nn(0,0,e,i),this.scissorTest=!1,this.viewport=new nn(0,0,e,i),this.textures=[];const l={width:e,height:i,depth:s.depth},c=new Fn(l),d=s.count;for(let p=0;p<d;p++)this.textures[p]=c.clone(),this.textures[p].isRenderTargetTexture=!0,this.textures[p].renderTarget=this;this._setTextureOptions(s),this.depthBuffer=s.depthBuffer,this.stencilBuffer=s.stencilBuffer,this.resolveDepthBuffer=s.resolveDepthBuffer,this.resolveStencilBuffer=s.resolveStencilBuffer,this._depthTexture=null,this.depthTexture=s.depthTexture,this.samples=s.samples,this.multiview=s.multiview}_setTextureOptions(e={}){const i={minFilter:Dn,generateMipmaps:!1,flipY:!1,internalFormat:null};e.mapping!==void 0&&(i.mapping=e.mapping),e.wrapS!==void 0&&(i.wrapS=e.wrapS),e.wrapT!==void 0&&(i.wrapT=e.wrapT),e.wrapR!==void 0&&(i.wrapR=e.wrapR),e.magFilter!==void 0&&(i.magFilter=e.magFilter),e.minFilter!==void 0&&(i.minFilter=e.minFilter),e.format!==void 0&&(i.format=e.format),e.type!==void 0&&(i.type=e.type),e.anisotropy!==void 0&&(i.anisotropy=e.anisotropy),e.colorSpace!==void 0&&(i.colorSpace=e.colorSpace),e.flipY!==void 0&&(i.flipY=e.flipY),e.generateMipmaps!==void 0&&(i.generateMipmaps=e.generateMipmaps),e.internalFormat!==void 0&&(i.internalFormat=e.internalFormat);for(let s=0;s<this.textures.length;s++)this.textures[s].setValues(i)}get texture(){return this.textures[0]}set texture(e){this.textures[0]=e}set depthTexture(e){this._depthTexture!==null&&(this._depthTexture.renderTarget=null),e!==null&&(e.renderTarget=this),this._depthTexture=e}get depthTexture(){return this._depthTexture}setSize(e,i,s=1){if(this.width!==e||this.height!==i||this.depth!==s){this.width=e,this.height=i,this.depth=s;for(let l=0,c=this.textures.length;l<c;l++)this.textures[l].image.width=e,this.textures[l].image.height=i,this.textures[l].image.depth=s,this.textures[l].isData3DTexture!==!0&&(this.textures[l].isArrayTexture=this.textures[l].image.depth>1);this.dispose()}this.viewport.set(0,0,e,i),this.scissor.set(0,0,e,i)}clone(){return new this.constructor().copy(this)}copy(e){this.width=e.width,this.height=e.height,this.depth=e.depth,this.scissor.copy(e.scissor),this.scissorTest=e.scissorTest,this.viewport.copy(e.viewport),this.textures.length=0;for(let i=0,s=e.textures.length;i<s;i++){this.textures[i]=e.textures[i].clone(),this.textures[i].isRenderTargetTexture=!0,this.textures[i].renderTarget=this;const l=Object.assign({},e.textures[i].image);this.textures[i].source=new Jh(l)}return this.depthBuffer=e.depthBuffer,this.stencilBuffer=e.stencilBuffer,this.resolveDepthBuffer=e.resolveDepthBuffer,this.resolveStencilBuffer=e.resolveStencilBuffer,e.depthTexture!==null&&(this.depthTexture=e.depthTexture.clone()),this.samples=e.samples,this}dispose(){this.dispatchEvent({type:"dispose"})}}class ki extends HS{constructor(e=1,i=1,s={}){super(e,i,s),this.isWebGLRenderTarget=!0}}class mv extends Fn{constructor(e=null,i=1,s=1,l=1){super(null),this.isDataArrayTexture=!0,this.image={data:e,width:i,height:s,depth:l},this.magFilter=An,this.minFilter=An,this.wrapR=xa,this.generateMipmaps=!1,this.flipY=!1,this.unpackAlignment=1,this.layerUpdates=new Set}addLayerUpdate(e){this.layerUpdates.add(e)}clearLayerUpdates(){this.layerUpdates.clear()}}class GS extends Fn{constructor(e=null,i=1,s=1,l=1){super(null),this.isData3DTexture=!0,this.image={data:e,width:i,height:s,depth:l},this.magFilter=An,this.minFilter=An,this.wrapR=xa,this.generateMipmaps=!1,this.flipY=!1,this.unpackAlignment=1}}class Jt{constructor(e,i,s,l,c,d,p,m,h,v,y,g,x,E,w,b){Jt.prototype.isMatrix4=!0,this.elements=[1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1],e!==void 0&&this.set(e,i,s,l,c,d,p,m,h,v,y,g,x,E,w,b)}set(e,i,s,l,c,d,p,m,h,v,y,g,x,E,w,b){const S=this.elements;return S[0]=e,S[4]=i,S[8]=s,S[12]=l,S[1]=c,S[5]=d,S[9]=p,S[13]=m,S[2]=h,S[6]=v,S[10]=y,S[14]=g,S[3]=x,S[7]=E,S[11]=w,S[15]=b,this}identity(){return this.set(1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1),this}clone(){return new Jt().fromArray(this.elements)}copy(e){const i=this.elements,s=e.elements;return i[0]=s[0],i[1]=s[1],i[2]=s[2],i[3]=s[3],i[4]=s[4],i[5]=s[5],i[6]=s[6],i[7]=s[7],i[8]=s[8],i[9]=s[9],i[10]=s[10],i[11]=s[11],i[12]=s[12],i[13]=s[13],i[14]=s[14],i[15]=s[15],this}copyPosition(e){const i=this.elements,s=e.elements;return i[12]=s[12],i[13]=s[13],i[14]=s[14],this}setFromMatrix3(e){const i=e.elements;return this.set(i[0],i[3],i[6],0,i[1],i[4],i[7],0,i[2],i[5],i[8],0,0,0,0,1),this}extractBasis(e,i,s){return this.determinant()===0?(e.set(1,0,0),i.set(0,1,0),s.set(0,0,1),this):(e.setFromMatrixColumn(this,0),i.setFromMatrixColumn(this,1),s.setFromMatrixColumn(this,2),this)}makeBasis(e,i,s){return this.set(e.x,i.x,s.x,0,e.y,i.y,s.y,0,e.z,i.z,s.z,0,0,0,0,1),this}extractRotation(e){if(e.determinant()===0)return this.identity();const i=this.elements,s=e.elements,l=1/br.setFromMatrixColumn(e,0).length(),c=1/br.setFromMatrixColumn(e,1).length(),d=1/br.setFromMatrixColumn(e,2).length();return i[0]=s[0]*l,i[1]=s[1]*l,i[2]=s[2]*l,i[3]=0,i[4]=s[4]*c,i[5]=s[5]*c,i[6]=s[6]*c,i[7]=0,i[8]=s[8]*d,i[9]=s[9]*d,i[10]=s[10]*d,i[11]=0,i[12]=0,i[13]=0,i[14]=0,i[15]=1,this}makeRotationFromEuler(e){const i=this.elements,s=e.x,l=e.y,c=e.z,d=Math.cos(s),p=Math.sin(s),m=Math.cos(l),h=Math.sin(l),v=Math.cos(c),y=Math.sin(c);if(e.order==="XYZ"){const g=d*v,x=d*y,E=p*v,w=p*y;i[0]=m*v,i[4]=-m*y,i[8]=h,i[1]=x+E*h,i[5]=g-w*h,i[9]=-p*m,i[2]=w-g*h,i[6]=E+x*h,i[10]=d*m}else if(e.order==="YXZ"){const g=m*v,x=m*y,E=h*v,w=h*y;i[0]=g+w*p,i[4]=E*p-x,i[8]=d*h,i[1]=d*y,i[5]=d*v,i[9]=-p,i[2]=x*p-E,i[6]=w+g*p,i[10]=d*m}else if(e.order==="ZXY"){const g=m*v,x=m*y,E=h*v,w=h*y;i[0]=g-w*p,i[4]=-d*y,i[8]=E+x*p,i[1]=x+E*p,i[5]=d*v,i[9]=w-g*p,i[2]=-d*h,i[6]=p,i[10]=d*m}else if(e.order==="ZYX"){const g=d*v,x=d*y,E=p*v,w=p*y;i[0]=m*v,i[4]=E*h-x,i[8]=g*h+w,i[1]=m*y,i[5]=w*h+g,i[9]=x*h-E,i[2]=-h,i[6]=p*m,i[10]=d*m}else if(e.order==="YZX"){const g=d*m,x=d*h,E=p*m,w=p*h;i[0]=m*v,i[4]=w-g*y,i[8]=E*y+x,i[1]=y,i[5]=d*v,i[9]=-p*v,i[2]=-h*v,i[6]=x*y+E,i[10]=g-w*y}else if(e.order==="XZY"){const g=d*m,x=d*h,E=p*m,w=p*h;i[0]=m*v,i[4]=-y,i[8]=h*v,i[1]=g*y+w,i[5]=d*v,i[9]=x*y-E,i[2]=E*y-x,i[6]=p*v,i[10]=w*y+g}return i[3]=0,i[7]=0,i[11]=0,i[12]=0,i[13]=0,i[14]=0,i[15]=1,this}makeRotationFromQuaternion(e){return this.compose(VS,e,kS)}lookAt(e,i,s){const l=this.elements;return ii.subVectors(e,i),ii.lengthSq()===0&&(ii.z=1),ii.normalize(),Ka.crossVectors(s,ii),Ka.lengthSq()===0&&(Math.abs(s.z)===1?ii.x+=1e-4:ii.z+=1e-4,ii.normalize(),Ka.crossVectors(s,ii)),Ka.normalize(),pc.crossVectors(ii,Ka),l[0]=Ka.x,l[4]=pc.x,l[8]=ii.x,l[1]=Ka.y,l[5]=pc.y,l[9]=ii.y,l[2]=Ka.z,l[6]=pc.z,l[10]=ii.z,this}multiply(e){return this.multiplyMatrices(this,e)}premultiply(e){return this.multiplyMatrices(e,this)}multiplyMatrices(e,i){const s=e.elements,l=i.elements,c=this.elements,d=s[0],p=s[4],m=s[8],h=s[12],v=s[1],y=s[5],g=s[9],x=s[13],E=s[2],w=s[6],b=s[10],S=s[14],C=s[3],U=s[7],N=s[11],V=s[15],H=l[0],F=l[4],T=l[8],D=l[12],le=l[1],G=l[5],te=l[9],se=l[13],ue=l[2],ee=l[6],P=l[10],z=l[14],ce=l[3],pe=l[7],Ee=l[11],I=l[15];return c[0]=d*H+p*le+m*ue+h*ce,c[4]=d*F+p*G+m*ee+h*pe,c[8]=d*T+p*te+m*P+h*Ee,c[12]=d*D+p*se+m*z+h*I,c[1]=v*H+y*le+g*ue+x*ce,c[5]=v*F+y*G+g*ee+x*pe,c[9]=v*T+y*te+g*P+x*Ee,c[13]=v*D+y*se+g*z+x*I,c[2]=E*H+w*le+b*ue+S*ce,c[6]=E*F+w*G+b*ee+S*pe,c[10]=E*T+w*te+b*P+S*Ee,c[14]=E*D+w*se+b*z+S*I,c[3]=C*H+U*le+N*ue+V*ce,c[7]=C*F+U*G+N*ee+V*pe,c[11]=C*T+U*te+N*P+V*Ee,c[15]=C*D+U*se+N*z+V*I,this}multiplyScalar(e){const i=this.elements;return i[0]*=e,i[4]*=e,i[8]*=e,i[12]*=e,i[1]*=e,i[5]*=e,i[9]*=e,i[13]*=e,i[2]*=e,i[6]*=e,i[10]*=e,i[14]*=e,i[3]*=e,i[7]*=e,i[11]*=e,i[15]*=e,this}determinant(){const e=this.elements,i=e[0],s=e[4],l=e[8],c=e[12],d=e[1],p=e[5],m=e[9],h=e[13],v=e[2],y=e[6],g=e[10],x=e[14],E=e[3],w=e[7],b=e[11],S=e[15],C=m*x-h*g,U=p*x-h*y,N=p*g-m*y,V=d*x-h*v,H=d*g-m*v,F=d*y-p*v;return i*(w*C-b*U+S*N)-s*(E*C-b*V+S*H)+l*(E*U-w*V+S*F)-c*(E*N-w*H+b*F)}transpose(){const e=this.elements;let i;return i=e[1],e[1]=e[4],e[4]=i,i=e[2],e[2]=e[8],e[8]=i,i=e[6],e[6]=e[9],e[9]=i,i=e[3],e[3]=e[12],e[12]=i,i=e[7],e[7]=e[13],e[13]=i,i=e[11],e[11]=e[14],e[14]=i,this}setPosition(e,i,s){const l=this.elements;return e.isVector3?(l[12]=e.x,l[13]=e.y,l[14]=e.z):(l[12]=e,l[13]=i,l[14]=s),this}invert(){const e=this.elements,i=e[0],s=e[1],l=e[2],c=e[3],d=e[4],p=e[5],m=e[6],h=e[7],v=e[8],y=e[9],g=e[10],x=e[11],E=e[12],w=e[13],b=e[14],S=e[15],C=i*p-s*d,U=i*m-l*d,N=i*h-c*d,V=s*m-l*p,H=s*h-c*p,F=l*h-c*m,T=v*w-y*E,D=v*b-g*E,le=v*S-x*E,G=y*b-g*w,te=y*S-x*w,se=g*S-x*b,ue=C*se-U*te+N*G+V*le-H*D+F*T;if(ue===0)return this.set(0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0);const ee=1/ue;return e[0]=(p*se-m*te+h*G)*ee,e[1]=(l*te-s*se-c*G)*ee,e[2]=(w*F-b*H+S*V)*ee,e[3]=(g*H-y*F-x*V)*ee,e[4]=(m*le-d*se-h*D)*ee,e[5]=(i*se-l*le+c*D)*ee,e[6]=(b*N-E*F-S*U)*ee,e[7]=(v*F-g*N+x*U)*ee,e[8]=(d*te-p*le+h*T)*ee,e[9]=(s*le-i*te-c*T)*ee,e[10]=(E*H-w*N+S*C)*ee,e[11]=(y*N-v*H-x*C)*ee,e[12]=(p*D-d*G-m*T)*ee,e[13]=(i*G-s*D+l*T)*ee,e[14]=(w*U-E*V-b*C)*ee,e[15]=(v*V-y*U+g*C)*ee,this}scale(e){const i=this.elements,s=e.x,l=e.y,c=e.z;return i[0]*=s,i[4]*=l,i[8]*=c,i[1]*=s,i[5]*=l,i[9]*=c,i[2]*=s,i[6]*=l,i[10]*=c,i[3]*=s,i[7]*=l,i[11]*=c,this}getMaxScaleOnAxis(){const e=this.elements,i=e[0]*e[0]+e[1]*e[1]+e[2]*e[2],s=e[4]*e[4]+e[5]*e[5]+e[6]*e[6],l=e[8]*e[8]+e[9]*e[9]+e[10]*e[10];return Math.sqrt(Math.max(i,s,l))}makeTranslation(e,i,s){return e.isVector3?this.set(1,0,0,e.x,0,1,0,e.y,0,0,1,e.z,0,0,0,1):this.set(1,0,0,e,0,1,0,i,0,0,1,s,0,0,0,1),this}makeRotationX(e){const i=Math.cos(e),s=Math.sin(e);return this.set(1,0,0,0,0,i,-s,0,0,s,i,0,0,0,0,1),this}makeRotationY(e){const i=Math.cos(e),s=Math.sin(e);return this.set(i,0,s,0,0,1,0,0,-s,0,i,0,0,0,0,1),this}makeRotationZ(e){const i=Math.cos(e),s=Math.sin(e);return this.set(i,-s,0,0,s,i,0,0,0,0,1,0,0,0,0,1),this}makeRotationAxis(e,i){const s=Math.cos(i),l=Math.sin(i),c=1-s,d=e.x,p=e.y,m=e.z,h=c*d,v=c*p;return this.set(h*d+s,h*p-l*m,h*m+l*p,0,h*p+l*m,v*p+s,v*m-l*d,0,h*m-l*p,v*m+l*d,c*m*m+s,0,0,0,0,1),this}makeScale(e,i,s){return this.set(e,0,0,0,0,i,0,0,0,0,s,0,0,0,0,1),this}makeShear(e,i,s,l,c,d){return this.set(1,s,c,0,e,1,d,0,i,l,1,0,0,0,0,1),this}compose(e,i,s){const l=this.elements,c=i._x,d=i._y,p=i._z,m=i._w,h=c+c,v=d+d,y=p+p,g=c*h,x=c*v,E=c*y,w=d*v,b=d*y,S=p*y,C=m*h,U=m*v,N=m*y,V=s.x,H=s.y,F=s.z;return l[0]=(1-(w+S))*V,l[1]=(x+N)*V,l[2]=(E-U)*V,l[3]=0,l[4]=(x-N)*H,l[5]=(1-(g+S))*H,l[6]=(b+C)*H,l[7]=0,l[8]=(E+U)*F,l[9]=(b-C)*F,l[10]=(1-(g+w))*F,l[11]=0,l[12]=e.x,l[13]=e.y,l[14]=e.z,l[15]=1,this}decompose(e,i,s){const l=this.elements;e.x=l[12],e.y=l[13],e.z=l[14];const c=this.determinant();if(c===0)return s.set(1,1,1),i.identity(),this;let d=br.set(l[0],l[1],l[2]).length();const p=br.set(l[4],l[5],l[6]).length(),m=br.set(l[8],l[9],l[10]).length();c<0&&(d=-d),Ai.copy(this);const h=1/d,v=1/p,y=1/m;return Ai.elements[0]*=h,Ai.elements[1]*=h,Ai.elements[2]*=h,Ai.elements[4]*=v,Ai.elements[5]*=v,Ai.elements[6]*=v,Ai.elements[8]*=y,Ai.elements[9]*=y,Ai.elements[10]*=y,i.setFromRotationMatrix(Ai),s.x=d,s.y=p,s.z=m,this}makePerspective(e,i,s,l,c,d,p=Gi,m=!1){const h=this.elements,v=2*c/(i-e),y=2*c/(s-l),g=(i+e)/(i-e),x=(s+l)/(s-l);let E,w;if(m)E=c/(d-c),w=d*c/(d-c);else if(p===Gi)E=-(d+c)/(d-c),w=-2*d*c/(d-c);else if(p===Zo)E=-d/(d-c),w=-d*c/(d-c);else throw new Error("THREE.Matrix4.makePerspective(): Invalid coordinate system: "+p);return h[0]=v,h[4]=0,h[8]=g,h[12]=0,h[1]=0,h[5]=y,h[9]=x,h[13]=0,h[2]=0,h[6]=0,h[10]=E,h[14]=w,h[3]=0,h[7]=0,h[11]=-1,h[15]=0,this}makeOrthographic(e,i,s,l,c,d,p=Gi,m=!1){const h=this.elements,v=2/(i-e),y=2/(s-l),g=-(i+e)/(i-e),x=-(s+l)/(s-l);let E,w;if(m)E=1/(d-c),w=d/(d-c);else if(p===Gi)E=-2/(d-c),w=-(d+c)/(d-c);else if(p===Zo)E=-1/(d-c),w=-c/(d-c);else throw new Error("THREE.Matrix4.makeOrthographic(): Invalid coordinate system: "+p);return h[0]=v,h[4]=0,h[8]=0,h[12]=g,h[1]=0,h[5]=y,h[9]=0,h[13]=x,h[2]=0,h[6]=0,h[10]=E,h[14]=w,h[3]=0,h[7]=0,h[11]=0,h[15]=1,this}equals(e){const i=this.elements,s=e.elements;for(let l=0;l<16;l++)if(i[l]!==s[l])return!1;return!0}fromArray(e,i=0){for(let s=0;s<16;s++)this.elements[s]=e[s+i];return this}toArray(e=[],i=0){const s=this.elements;return e[i]=s[0],e[i+1]=s[1],e[i+2]=s[2],e[i+3]=s[3],e[i+4]=s[4],e[i+5]=s[5],e[i+6]=s[6],e[i+7]=s[7],e[i+8]=s[8],e[i+9]=s[9],e[i+10]=s[10],e[i+11]=s[11],e[i+12]=s[12],e[i+13]=s[13],e[i+14]=s[14],e[i+15]=s[15],e}}const br=new K,Ai=new Jt,VS=new K(0,0,0),kS=new K(1,1,1),Ka=new K,pc=new K,ii=new K,i_=new Jt,a_=new rs;class ji{constructor(e=0,i=0,s=0,l=ji.DEFAULT_ORDER){this.isEuler=!0,this._x=e,this._y=i,this._z=s,this._order=l}get x(){return this._x}set x(e){this._x=e,this._onChangeCallback()}get y(){return this._y}set y(e){this._y=e,this._onChangeCallback()}get z(){return this._z}set z(e){this._z=e,this._onChangeCallback()}get order(){return this._order}set order(e){this._order=e,this._onChangeCallback()}set(e,i,s,l=this._order){return this._x=e,this._y=i,this._z=s,this._order=l,this._onChangeCallback(),this}clone(){return new this.constructor(this._x,this._y,this._z,this._order)}copy(e){return this._x=e._x,this._y=e._y,this._z=e._z,this._order=e._order,this._onChangeCallback(),this}setFromRotationMatrix(e,i=this._order,s=!0){const l=e.elements,c=l[0],d=l[4],p=l[8],m=l[1],h=l[5],v=l[9],y=l[2],g=l[6],x=l[10];switch(i){case"XYZ":this._y=Math.asin(vt(p,-1,1)),Math.abs(p)<.9999999?(this._x=Math.atan2(-v,x),this._z=Math.atan2(-d,c)):(this._x=Math.atan2(g,h),this._z=0);break;case"YXZ":this._x=Math.asin(-vt(v,-1,1)),Math.abs(v)<.9999999?(this._y=Math.atan2(p,x),this._z=Math.atan2(m,h)):(this._y=Math.atan2(-y,c),this._z=0);break;case"ZXY":this._x=Math.asin(vt(g,-1,1)),Math.abs(g)<.9999999?(this._y=Math.atan2(-y,x),this._z=Math.atan2(-d,h)):(this._y=0,this._z=Math.atan2(m,c));break;case"ZYX":this._y=Math.asin(-vt(y,-1,1)),Math.abs(y)<.9999999?(this._x=Math.atan2(g,x),this._z=Math.atan2(m,c)):(this._x=0,this._z=Math.atan2(-d,h));break;case"YZX":this._z=Math.asin(vt(m,-1,1)),Math.abs(m)<.9999999?(this._x=Math.atan2(-v,h),this._y=Math.atan2(-y,c)):(this._x=0,this._y=Math.atan2(p,x));break;case"XZY":this._z=Math.asin(-vt(d,-1,1)),Math.abs(d)<.9999999?(this._x=Math.atan2(g,h),this._y=Math.atan2(p,c)):(this._x=Math.atan2(-v,x),this._y=0);break;default:at("Euler: .setFromRotationMatrix() encountered an unknown order: "+i)}return this._order=i,s===!0&&this._onChangeCallback(),this}setFromQuaternion(e,i,s){return i_.makeRotationFromQuaternion(e),this.setFromRotationMatrix(i_,i,s)}setFromVector3(e,i=this._order){return this.set(e.x,e.y,e.z,i)}reorder(e){return a_.setFromEuler(this),this.setFromQuaternion(a_,e)}equals(e){return e._x===this._x&&e._y===this._y&&e._z===this._z&&e._order===this._order}fromArray(e){return this._x=e[0],this._y=e[1],this._z=e[2],e[3]!==void 0&&(this._order=e[3]),this._onChangeCallback(),this}toArray(e=[],i=0){return e[i]=this._x,e[i+1]=this._y,e[i+2]=this._z,e[i+3]=this._order,e}_onChange(e){return this._onChangeCallback=e,this}_onChangeCallback(){}*[Symbol.iterator](){yield this._x,yield this._y,yield this._z,yield this._order}}ji.DEFAULT_ORDER="XYZ";class gv{constructor(){this.mask=1}set(e){this.mask=(1<<e|0)>>>0}enable(e){this.mask|=1<<e|0}enableAll(){this.mask=-1}toggle(e){this.mask^=1<<e|0}disable(e){this.mask&=~(1<<e|0)}disableAll(){this.mask=0}test(e){return(this.mask&e.mask)!==0}isEnabled(e){return(this.mask&(1<<e|0))!==0}}let XS=0;const s_=new K,Mr=new rs,ha=new Jt,mc=new K,Bo=new K,jS=new K,WS=new rs,r_=new K(1,0,0),o_=new K(0,1,0),l_=new K(0,0,1),c_={type:"added"},qS={type:"removed"},Er={type:"childadded",child:null},Sd={type:"childremoved",child:null};class zn extends Fs{constructor(){super(),this.isObject3D=!0,Object.defineProperty(this,"id",{value:XS++}),this.uuid=Qo(),this.name="",this.type="Object3D",this.parent=null,this.children=[],this.up=zn.DEFAULT_UP.clone();const e=new K,i=new ji,s=new rs,l=new K(1,1,1);function c(){s.setFromEuler(i,!1)}function d(){i.setFromQuaternion(s,void 0,!1)}i._onChange(c),s._onChange(d),Object.defineProperties(this,{position:{configurable:!0,enumerable:!0,value:e},rotation:{configurable:!0,enumerable:!0,value:i},quaternion:{configurable:!0,enumerable:!0,value:s},scale:{configurable:!0,enumerable:!0,value:l},modelViewMatrix:{value:new Jt},normalMatrix:{value:new ht}}),this.matrix=new Jt,this.matrixWorld=new Jt,this.matrixAutoUpdate=zn.DEFAULT_MATRIX_AUTO_UPDATE,this.matrixWorldAutoUpdate=zn.DEFAULT_MATRIX_WORLD_AUTO_UPDATE,this.matrixWorldNeedsUpdate=!1,this.layers=new gv,this.visible=!0,this.castShadow=!1,this.receiveShadow=!1,this.frustumCulled=!0,this.renderOrder=0,this.animations=[],this.customDepthMaterial=void 0,this.customDistanceMaterial=void 0,this.static=!1,this.userData={},this.pivot=null}onBeforeShadow(){}onAfterShadow(){}onBeforeRender(){}onAfterRender(){}applyMatrix4(e){this.matrixAutoUpdate&&this.updateMatrix(),this.matrix.premultiply(e),this.matrix.decompose(this.position,this.quaternion,this.scale)}applyQuaternion(e){return this.quaternion.premultiply(e),this}setRotationFromAxisAngle(e,i){this.quaternion.setFromAxisAngle(e,i)}setRotationFromEuler(e){this.quaternion.setFromEuler(e,!0)}setRotationFromMatrix(e){this.quaternion.setFromRotationMatrix(e)}setRotationFromQuaternion(e){this.quaternion.copy(e)}rotateOnAxis(e,i){return Mr.setFromAxisAngle(e,i),this.quaternion.multiply(Mr),this}rotateOnWorldAxis(e,i){return Mr.setFromAxisAngle(e,i),this.quaternion.premultiply(Mr),this}rotateX(e){return this.rotateOnAxis(r_,e)}rotateY(e){return this.rotateOnAxis(o_,e)}rotateZ(e){return this.rotateOnAxis(l_,e)}translateOnAxis(e,i){return s_.copy(e).applyQuaternion(this.quaternion),this.position.add(s_.multiplyScalar(i)),this}translateX(e){return this.translateOnAxis(r_,e)}translateY(e){return this.translateOnAxis(o_,e)}translateZ(e){return this.translateOnAxis(l_,e)}localToWorld(e){return this.updateWorldMatrix(!0,!1),e.applyMatrix4(this.matrixWorld)}worldToLocal(e){return this.updateWorldMatrix(!0,!1),e.applyMatrix4(ha.copy(this.matrixWorld).invert())}lookAt(e,i,s){e.isVector3?mc.copy(e):mc.set(e,i,s);const l=this.parent;this.updateWorldMatrix(!0,!1),Bo.setFromMatrixPosition(this.matrixWorld),this.isCamera||this.isLight?ha.lookAt(Bo,mc,this.up):ha.lookAt(mc,Bo,this.up),this.quaternion.setFromRotationMatrix(ha),l&&(ha.extractRotation(l.matrixWorld),Mr.setFromRotationMatrix(ha),this.quaternion.premultiply(Mr.invert()))}add(e){if(arguments.length>1){for(let i=0;i<arguments.length;i++)this.add(arguments[i]);return this}return e===this?(Dt("Object3D.add: object can't be added as a child of itself.",e),this):(e&&e.isObject3D?(e.removeFromParent(),e.parent=this,this.children.push(e),e.dispatchEvent(c_),Er.child=e,this.dispatchEvent(Er),Er.child=null):Dt("Object3D.add: object not an instance of THREE.Object3D.",e),this)}remove(e){if(arguments.length>1){for(let s=0;s<arguments.length;s++)this.remove(arguments[s]);return this}const i=this.children.indexOf(e);return i!==-1&&(e.parent=null,this.children.splice(i,1),e.dispatchEvent(qS),Sd.child=e,this.dispatchEvent(Sd),Sd.child=null),this}removeFromParent(){const e=this.parent;return e!==null&&e.remove(this),this}clear(){return this.remove(...this.children)}attach(e){return this.updateWorldMatrix(!0,!1),ha.copy(this.matrixWorld).invert(),e.parent!==null&&(e.parent.updateWorldMatrix(!0,!1),ha.multiply(e.parent.matrixWorld)),e.applyMatrix4(ha),e.removeFromParent(),e.parent=this,this.children.push(e),e.updateWorldMatrix(!1,!0),e.dispatchEvent(c_),Er.child=e,this.dispatchEvent(Er),Er.child=null,this}getObjectById(e){return this.getObjectByProperty("id",e)}getObjectByName(e){return this.getObjectByProperty("name",e)}getObjectByProperty(e,i){if(this[e]===i)return this;for(let s=0,l=this.children.length;s<l;s++){const d=this.children[s].getObjectByProperty(e,i);if(d!==void 0)return d}}getObjectsByProperty(e,i,s=[]){this[e]===i&&s.push(this);const l=this.children;for(let c=0,d=l.length;c<d;c++)l[c].getObjectsByProperty(e,i,s);return s}getWorldPosition(e){return this.updateWorldMatrix(!0,!1),e.setFromMatrixPosition(this.matrixWorld)}getWorldQuaternion(e){return this.updateWorldMatrix(!0,!1),this.matrixWorld.decompose(Bo,e,jS),e}getWorldScale(e){return this.updateWorldMatrix(!0,!1),this.matrixWorld.decompose(Bo,WS,e),e}getWorldDirection(e){this.updateWorldMatrix(!0,!1);const i=this.matrixWorld.elements;return e.set(i[8],i[9],i[10]).normalize()}raycast(){}traverse(e){e(this);const i=this.children;for(let s=0,l=i.length;s<l;s++)i[s].traverse(e)}traverseVisible(e){if(this.visible===!1)return;e(this);const i=this.children;for(let s=0,l=i.length;s<l;s++)i[s].traverseVisible(e)}traverseAncestors(e){const i=this.parent;i!==null&&(e(i),i.traverseAncestors(e))}updateMatrix(){this.matrix.compose(this.position,this.quaternion,this.scale);const e=this.pivot;if(e!==null){const i=e.x,s=e.y,l=e.z,c=this.matrix.elements;c[12]+=i-c[0]*i-c[4]*s-c[8]*l,c[13]+=s-c[1]*i-c[5]*s-c[9]*l,c[14]+=l-c[2]*i-c[6]*s-c[10]*l}this.matrixWorldNeedsUpdate=!0}updateMatrixWorld(e){this.matrixAutoUpdate&&this.updateMatrix(),(this.matrixWorldNeedsUpdate||e)&&(this.matrixWorldAutoUpdate===!0&&(this.parent===null?this.matrixWorld.copy(this.matrix):this.matrixWorld.multiplyMatrices(this.parent.matrixWorld,this.matrix)),this.matrixWorldNeedsUpdate=!1,e=!0);const i=this.children;for(let s=0,l=i.length;s<l;s++)i[s].updateMatrixWorld(e)}updateWorldMatrix(e,i){const s=this.parent;if(e===!0&&s!==null&&s.updateWorldMatrix(!0,!1),this.matrixAutoUpdate&&this.updateMatrix(),this.matrixWorldAutoUpdate===!0&&(this.parent===null?this.matrixWorld.copy(this.matrix):this.matrixWorld.multiplyMatrices(this.parent.matrixWorld,this.matrix)),i===!0){const l=this.children;for(let c=0,d=l.length;c<d;c++)l[c].updateWorldMatrix(!1,!0)}}toJSON(e){const i=e===void 0||typeof e=="string",s={};i&&(e={geometries:{},materials:{},textures:{},images:{},shapes:{},skeletons:{},animations:{},nodes:{}},s.metadata={version:4.7,type:"Object",generator:"Object3D.toJSON"});const l={};l.uuid=this.uuid,l.type=this.type,this.name!==""&&(l.name=this.name),this.castShadow===!0&&(l.castShadow=!0),this.receiveShadow===!0&&(l.receiveShadow=!0),this.visible===!1&&(l.visible=!1),this.frustumCulled===!1&&(l.frustumCulled=!1),this.renderOrder!==0&&(l.renderOrder=this.renderOrder),this.static!==!1&&(l.static=this.static),Object.keys(this.userData).length>0&&(l.userData=this.userData),l.layers=this.layers.mask,l.matrix=this.matrix.toArray(),l.up=this.up.toArray(),this.pivot!==null&&(l.pivot=this.pivot.toArray()),this.matrixAutoUpdate===!1&&(l.matrixAutoUpdate=!1),this.morphTargetDictionary!==void 0&&(l.morphTargetDictionary=Object.assign({},this.morphTargetDictionary)),this.morphTargetInfluences!==void 0&&(l.morphTargetInfluences=this.morphTargetInfluences.slice()),this.isInstancedMesh&&(l.type="InstancedMesh",l.count=this.count,l.instanceMatrix=this.instanceMatrix.toJSON(),this.instanceColor!==null&&(l.instanceColor=this.instanceColor.toJSON())),this.isBatchedMesh&&(l.type="BatchedMesh",l.perObjectFrustumCulled=this.perObjectFrustumCulled,l.sortObjects=this.sortObjects,l.drawRanges=this._drawRanges,l.reservedRanges=this._reservedRanges,l.geometryInfo=this._geometryInfo.map(p=>({...p,boundingBox:p.boundingBox?p.boundingBox.toJSON():void 0,boundingSphere:p.boundingSphere?p.boundingSphere.toJSON():void 0})),l.instanceInfo=this._instanceInfo.map(p=>({...p})),l.availableInstanceIds=this._availableInstanceIds.slice(),l.availableGeometryIds=this._availableGeometryIds.slice(),l.nextIndexStart=this._nextIndexStart,l.nextVertexStart=this._nextVertexStart,l.geometryCount=this._geometryCount,l.maxInstanceCount=this._maxInstanceCount,l.maxVertexCount=this._maxVertexCount,l.maxIndexCount=this._maxIndexCount,l.geometryInitialized=this._geometryInitialized,l.matricesTexture=this._matricesTexture.toJSON(e),l.indirectTexture=this._indirectTexture.toJSON(e),this._colorsTexture!==null&&(l.colorsTexture=this._colorsTexture.toJSON(e)),this.boundingSphere!==null&&(l.boundingSphere=this.boundingSphere.toJSON()),this.boundingBox!==null&&(l.boundingBox=this.boundingBox.toJSON()));function c(p,m){return p[m.uuid]===void 0&&(p[m.uuid]=m.toJSON(e)),m.uuid}if(this.isScene)this.background&&(this.background.isColor?l.background=this.background.toJSON():this.background.isTexture&&(l.background=this.background.toJSON(e).uuid)),this.environment&&this.environment.isTexture&&this.environment.isRenderTargetTexture!==!0&&(l.environment=this.environment.toJSON(e).uuid);else if(this.isMesh||this.isLine||this.isPoints){l.geometry=c(e.geometries,this.geometry);const p=this.geometry.parameters;if(p!==void 0&&p.shapes!==void 0){const m=p.shapes;if(Array.isArray(m))for(let h=0,v=m.length;h<v;h++){const y=m[h];c(e.shapes,y)}else c(e.shapes,m)}}if(this.isSkinnedMesh&&(l.bindMode=this.bindMode,l.bindMatrix=this.bindMatrix.toArray(),this.skeleton!==void 0&&(c(e.skeletons,this.skeleton),l.skeleton=this.skeleton.uuid)),this.material!==void 0)if(Array.isArray(this.material)){const p=[];for(let m=0,h=this.material.length;m<h;m++)p.push(c(e.materials,this.material[m]));l.material=p}else l.material=c(e.materials,this.material);if(this.children.length>0){l.children=[];for(let p=0;p<this.children.length;p++)l.children.push(this.children[p].toJSON(e).object)}if(this.animations.length>0){l.animations=[];for(let p=0;p<this.animations.length;p++){const m=this.animations[p];l.animations.push(c(e.animations,m))}}if(i){const p=d(e.geometries),m=d(e.materials),h=d(e.textures),v=d(e.images),y=d(e.shapes),g=d(e.skeletons),x=d(e.animations),E=d(e.nodes);p.length>0&&(s.geometries=p),m.length>0&&(s.materials=m),h.length>0&&(s.textures=h),v.length>0&&(s.images=v),y.length>0&&(s.shapes=y),g.length>0&&(s.skeletons=g),x.length>0&&(s.animations=x),E.length>0&&(s.nodes=E)}return s.object=l,s;function d(p){const m=[];for(const h in p){const v=p[h];delete v.metadata,m.push(v)}return m}}clone(e){return new this.constructor().copy(this,e)}copy(e,i=!0){if(this.name=e.name,this.up.copy(e.up),this.position.copy(e.position),this.rotation.order=e.rotation.order,this.quaternion.copy(e.quaternion),this.scale.copy(e.scale),e.pivot!==null&&(this.pivot=e.pivot.clone()),this.matrix.copy(e.matrix),this.matrixWorld.copy(e.matrixWorld),this.matrixAutoUpdate=e.matrixAutoUpdate,this.matrixWorldAutoUpdate=e.matrixWorldAutoUpdate,this.matrixWorldNeedsUpdate=e.matrixWorldNeedsUpdate,this.layers.mask=e.layers.mask,this.visible=e.visible,this.castShadow=e.castShadow,this.receiveShadow=e.receiveShadow,this.frustumCulled=e.frustumCulled,this.renderOrder=e.renderOrder,this.static=e.static,this.animations=e.animations.slice(),this.userData=JSON.parse(JSON.stringify(e.userData)),i===!0)for(let s=0;s<e.children.length;s++){const l=e.children[s];this.add(l.clone())}return this}}zn.DEFAULT_UP=new K(0,1,0);zn.DEFAULT_MATRIX_AUTO_UPDATE=!0;zn.DEFAULT_MATRIX_WORLD_AUTO_UPDATE=!0;class gc extends zn{constructor(){super(),this.isGroup=!0,this.type="Group"}}const YS={type:"move"};class bd{constructor(){this._targetRay=null,this._grip=null,this._hand=null}getHandSpace(){return this._hand===null&&(this._hand=new gc,this._hand.matrixAutoUpdate=!1,this._hand.visible=!1,this._hand.joints={},this._hand.inputState={pinching:!1}),this._hand}getTargetRaySpace(){return this._targetRay===null&&(this._targetRay=new gc,this._targetRay.matrixAutoUpdate=!1,this._targetRay.visible=!1,this._targetRay.hasLinearVelocity=!1,this._targetRay.linearVelocity=new K,this._targetRay.hasAngularVelocity=!1,this._targetRay.angularVelocity=new K),this._targetRay}getGripSpace(){return this._grip===null&&(this._grip=new gc,this._grip.matrixAutoUpdate=!1,this._grip.visible=!1,this._grip.hasLinearVelocity=!1,this._grip.linearVelocity=new K,this._grip.hasAngularVelocity=!1,this._grip.angularVelocity=new K),this._grip}dispatchEvent(e){return this._targetRay!==null&&this._targetRay.dispatchEvent(e),this._grip!==null&&this._grip.dispatchEvent(e),this._hand!==null&&this._hand.dispatchEvent(e),this}connect(e){if(e&&e.hand){const i=this._hand;if(i)for(const s of e.hand.values())this._getHandJoint(i,s)}return this.dispatchEvent({type:"connected",data:e}),this}disconnect(e){return this.dispatchEvent({type:"disconnected",data:e}),this._targetRay!==null&&(this._targetRay.visible=!1),this._grip!==null&&(this._grip.visible=!1),this._hand!==null&&(this._hand.visible=!1),this}update(e,i,s){let l=null,c=null,d=null;const p=this._targetRay,m=this._grip,h=this._hand;if(e&&i.session.visibilityState!=="visible-blurred"){if(h&&e.hand){d=!0;for(const w of e.hand.values()){const b=i.getJointPose(w,s),S=this._getHandJoint(h,w);b!==null&&(S.matrix.fromArray(b.transform.matrix),S.matrix.decompose(S.position,S.rotation,S.scale),S.matrixWorldNeedsUpdate=!0,S.jointRadius=b.radius),S.visible=b!==null}const v=h.joints["index-finger-tip"],y=h.joints["thumb-tip"],g=v.position.distanceTo(y.position),x=.02,E=.005;h.inputState.pinching&&g>x+E?(h.inputState.pinching=!1,this.dispatchEvent({type:"pinchend",handedness:e.handedness,target:this})):!h.inputState.pinching&&g<=x-E&&(h.inputState.pinching=!0,this.dispatchEvent({type:"pinchstart",handedness:e.handedness,target:this}))}else m!==null&&e.gripSpace&&(c=i.getPose(e.gripSpace,s),c!==null&&(m.matrix.fromArray(c.transform.matrix),m.matrix.decompose(m.position,m.rotation,m.scale),m.matrixWorldNeedsUpdate=!0,c.linearVelocity?(m.hasLinearVelocity=!0,m.linearVelocity.copy(c.linearVelocity)):m.hasLinearVelocity=!1,c.angularVelocity?(m.hasAngularVelocity=!0,m.angularVelocity.copy(c.angularVelocity)):m.hasAngularVelocity=!1));p!==null&&(l=i.getPose(e.targetRaySpace,s),l===null&&c!==null&&(l=c),l!==null&&(p.matrix.fromArray(l.transform.matrix),p.matrix.decompose(p.position,p.rotation,p.scale),p.matrixWorldNeedsUpdate=!0,l.linearVelocity?(p.hasLinearVelocity=!0,p.linearVelocity.copy(l.linearVelocity)):p.hasLinearVelocity=!1,l.angularVelocity?(p.hasAngularVelocity=!0,p.angularVelocity.copy(l.angularVelocity)):p.hasAngularVelocity=!1,this.dispatchEvent(YS)))}return p!==null&&(p.visible=l!==null),m!==null&&(m.visible=c!==null),h!==null&&(h.visible=d!==null),this}_getHandJoint(e,i){if(e.joints[i.jointName]===void 0){const s=new gc;s.matrixAutoUpdate=!1,s.visible=!1,e.joints[i.jointName]=s,e.add(s)}return e.joints[i.jointName]}}const _v={aliceblue:15792383,antiquewhite:16444375,aqua:65535,aquamarine:8388564,azure:15794175,beige:16119260,bisque:16770244,black:0,blanchedalmond:16772045,blue:255,blueviolet:9055202,brown:10824234,burlywood:14596231,cadetblue:6266528,chartreuse:8388352,chocolate:13789470,coral:16744272,cornflowerblue:6591981,cornsilk:16775388,crimson:14423100,cyan:65535,darkblue:139,darkcyan:35723,darkgoldenrod:12092939,darkgray:11119017,darkgreen:25600,darkgrey:11119017,darkkhaki:12433259,darkmagenta:9109643,darkolivegreen:5597999,darkorange:16747520,darkorchid:10040012,darkred:9109504,darksalmon:15308410,darkseagreen:9419919,darkslateblue:4734347,darkslategray:3100495,darkslategrey:3100495,darkturquoise:52945,darkviolet:9699539,deeppink:16716947,deepskyblue:49151,dimgray:6908265,dimgrey:6908265,dodgerblue:2003199,firebrick:11674146,floralwhite:16775920,forestgreen:2263842,fuchsia:16711935,gainsboro:14474460,ghostwhite:16316671,gold:16766720,goldenrod:14329120,gray:8421504,green:32768,greenyellow:11403055,grey:8421504,honeydew:15794160,hotpink:16738740,indianred:13458524,indigo:4915330,ivory:16777200,khaki:15787660,lavender:15132410,lavenderblush:16773365,lawngreen:8190976,lemonchiffon:16775885,lightblue:11393254,lightcoral:15761536,lightcyan:14745599,lightgoldenrodyellow:16448210,lightgray:13882323,lightgreen:9498256,lightgrey:13882323,lightpink:16758465,lightsalmon:16752762,lightseagreen:2142890,lightskyblue:8900346,lightslategray:7833753,lightslategrey:7833753,lightsteelblue:11584734,lightyellow:16777184,lime:65280,limegreen:3329330,linen:16445670,magenta:16711935,maroon:8388608,mediumaquamarine:6737322,mediumblue:205,mediumorchid:12211667,mediumpurple:9662683,mediumseagreen:3978097,mediumslateblue:8087790,mediumspringgreen:64154,mediumturquoise:4772300,mediumvioletred:13047173,midnightblue:1644912,mintcream:16121850,mistyrose:16770273,moccasin:16770229,navajowhite:16768685,navy:128,oldlace:16643558,olive:8421376,olivedrab:7048739,orange:16753920,orangered:16729344,orchid:14315734,palegoldenrod:15657130,palegreen:10025880,paleturquoise:11529966,palevioletred:14381203,papayawhip:16773077,peachpuff:16767673,peru:13468991,pink:16761035,plum:14524637,powderblue:11591910,purple:8388736,rebeccapurple:6697881,red:16711680,rosybrown:12357519,royalblue:4286945,saddlebrown:9127187,salmon:16416882,sandybrown:16032864,seagreen:3050327,seashell:16774638,sienna:10506797,silver:12632256,skyblue:8900331,slateblue:6970061,slategray:7372944,slategrey:7372944,snow:16775930,springgreen:65407,steelblue:4620980,tan:13808780,teal:32896,thistle:14204888,tomato:16737095,turquoise:4251856,violet:15631086,wheat:16113331,white:16777215,whitesmoke:16119285,yellow:16776960,yellowgreen:10145074},Qa={h:0,s:0,l:0},_c={h:0,s:0,l:0};function Md(o,e,i){return i<0&&(i+=1),i>1&&(i-=1),i<1/6?o+(e-o)*6*i:i<1/2?e:i<2/3?o+(e-o)*6*(2/3-i):o}class At{constructor(e,i,s){return this.isColor=!0,this.r=1,this.g=1,this.b=1,this.set(e,i,s)}set(e,i,s){if(i===void 0&&s===void 0){const l=e;l&&l.isColor?this.copy(l):typeof l=="number"?this.setHex(l):typeof l=="string"&&this.setStyle(l)}else this.setRGB(e,i,s);return this}setScalar(e){return this.r=e,this.g=e,this.b=e,this}setHex(e,i=gi){return e=Math.floor(e),this.r=(e>>16&255)/255,this.g=(e>>8&255)/255,this.b=(e&255)/255,Tt.colorSpaceToWorking(this,i),this}setRGB(e,i,s,l=Tt.workingColorSpace){return this.r=e,this.g=i,this.b=s,Tt.colorSpaceToWorking(this,l),this}setHSL(e,i,s,l=Tt.workingColorSpace){if(e=OS(e,1),i=vt(i,0,1),s=vt(s,0,1),i===0)this.r=this.g=this.b=s;else{const c=s<=.5?s*(1+i):s+i-s*i,d=2*s-c;this.r=Md(d,c,e+1/3),this.g=Md(d,c,e),this.b=Md(d,c,e-1/3)}return Tt.colorSpaceToWorking(this,l),this}setStyle(e,i=gi){function s(c){c!==void 0&&parseFloat(c)<1&&at("Color: Alpha component of "+e+" will be ignored.")}let l;if(l=/^(\w+)\(([^\)]*)\)/.exec(e)){let c;const d=l[1],p=l[2];switch(d){case"rgb":case"rgba":if(c=/^\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*(?:,\s*(\d*\.?\d+)\s*)?$/.exec(p))return s(c[4]),this.setRGB(Math.min(255,parseInt(c[1],10))/255,Math.min(255,parseInt(c[2],10))/255,Math.min(255,parseInt(c[3],10))/255,i);if(c=/^\s*(\d+)\%\s*,\s*(\d+)\%\s*,\s*(\d+)\%\s*(?:,\s*(\d*\.?\d+)\s*)?$/.exec(p))return s(c[4]),this.setRGB(Math.min(100,parseInt(c[1],10))/100,Math.min(100,parseInt(c[2],10))/100,Math.min(100,parseInt(c[3],10))/100,i);break;case"hsl":case"hsla":if(c=/^\s*(\d*\.?\d+)\s*,\s*(\d*\.?\d+)\%\s*,\s*(\d*\.?\d+)\%\s*(?:,\s*(\d*\.?\d+)\s*)?$/.exec(p))return s(c[4]),this.setHSL(parseFloat(c[1])/360,parseFloat(c[2])/100,parseFloat(c[3])/100,i);break;default:at("Color: Unknown color model "+e)}}else if(l=/^\#([A-Fa-f\d]+)$/.exec(e)){const c=l[1],d=c.length;if(d===3)return this.setRGB(parseInt(c.charAt(0),16)/15,parseInt(c.charAt(1),16)/15,parseInt(c.charAt(2),16)/15,i);if(d===6)return this.setHex(parseInt(c,16),i);at("Color: Invalid hex color "+e)}else if(e&&e.length>0)return this.setColorName(e,i);return this}setColorName(e,i=gi){const s=_v[e.toLowerCase()];return s!==void 0?this.setHex(s,i):at("Color: Unknown color "+e),this}clone(){return new this.constructor(this.r,this.g,this.b)}copy(e){return this.r=e.r,this.g=e.g,this.b=e.b,this}copySRGBToLinear(e){return this.r=Sa(e.r),this.g=Sa(e.g),this.b=Sa(e.b),this}copyLinearToSRGB(e){return this.r=zr(e.r),this.g=zr(e.g),this.b=zr(e.b),this}convertSRGBToLinear(){return this.copySRGBToLinear(this),this}convertLinearToSRGB(){return this.copyLinearToSRGB(this),this}getHex(e=gi){return Tt.workingToColorSpace(Cn.copy(this),e),Math.round(vt(Cn.r*255,0,255))*65536+Math.round(vt(Cn.g*255,0,255))*256+Math.round(vt(Cn.b*255,0,255))}getHexString(e=gi){return("000000"+this.getHex(e).toString(16)).slice(-6)}getHSL(e,i=Tt.workingColorSpace){Tt.workingToColorSpace(Cn.copy(this),i);const s=Cn.r,l=Cn.g,c=Cn.b,d=Math.max(s,l,c),p=Math.min(s,l,c);let m,h;const v=(p+d)/2;if(p===d)m=0,h=0;else{const y=d-p;switch(h=v<=.5?y/(d+p):y/(2-d-p),d){case s:m=(l-c)/y+(l<c?6:0);break;case l:m=(c-s)/y+2;break;case c:m=(s-l)/y+4;break}m/=6}return e.h=m,e.s=h,e.l=v,e}getRGB(e,i=Tt.workingColorSpace){return Tt.workingToColorSpace(Cn.copy(this),i),e.r=Cn.r,e.g=Cn.g,e.b=Cn.b,e}getStyle(e=gi){Tt.workingToColorSpace(Cn.copy(this),e);const i=Cn.r,s=Cn.g,l=Cn.b;return e!==gi?`color(${e} ${i.toFixed(3)} ${s.toFixed(3)} ${l.toFixed(3)})`:`rgb(${Math.round(i*255)},${Math.round(s*255)},${Math.round(l*255)})`}offsetHSL(e,i,s){return this.getHSL(Qa),this.setHSL(Qa.h+e,Qa.s+i,Qa.l+s)}add(e){return this.r+=e.r,this.g+=e.g,this.b+=e.b,this}addColors(e,i){return this.r=e.r+i.r,this.g=e.g+i.g,this.b=e.b+i.b,this}addScalar(e){return this.r+=e,this.g+=e,this.b+=e,this}sub(e){return this.r=Math.max(0,this.r-e.r),this.g=Math.max(0,this.g-e.g),this.b=Math.max(0,this.b-e.b),this}multiply(e){return this.r*=e.r,this.g*=e.g,this.b*=e.b,this}multiplyScalar(e){return this.r*=e,this.g*=e,this.b*=e,this}lerp(e,i){return this.r+=(e.r-this.r)*i,this.g+=(e.g-this.g)*i,this.b+=(e.b-this.b)*i,this}lerpColors(e,i,s){return this.r=e.r+(i.r-e.r)*s,this.g=e.g+(i.g-e.g)*s,this.b=e.b+(i.b-e.b)*s,this}lerpHSL(e,i){this.getHSL(Qa),e.getHSL(_c);const s=gd(Qa.h,_c.h,i),l=gd(Qa.s,_c.s,i),c=gd(Qa.l,_c.l,i);return this.setHSL(s,l,c),this}setFromVector3(e){return this.r=e.x,this.g=e.y,this.b=e.z,this}applyMatrix3(e){const i=this.r,s=this.g,l=this.b,c=e.elements;return this.r=c[0]*i+c[3]*s+c[6]*l,this.g=c[1]*i+c[4]*s+c[7]*l,this.b=c[2]*i+c[5]*s+c[8]*l,this}equals(e){return e.r===this.r&&e.g===this.g&&e.b===this.b}fromArray(e,i=0){return this.r=e[i],this.g=e[i+1],this.b=e[i+2],this}toArray(e=[],i=0){return e[i]=this.r,e[i+1]=this.g,e[i+2]=this.b,e}fromBufferAttribute(e,i){return this.r=e.getX(i),this.g=e.getY(i),this.b=e.getZ(i),this}toJSON(){return this.getHex()}*[Symbol.iterator](){yield this.r,yield this.g,yield this.b}}const Cn=new At;At.NAMES=_v;class $h{constructor(e,i=25e-5){this.isFogExp2=!0,this.name="",this.color=new At(e),this.density=i}clone(){return new $h(this.color,this.density)}toJSON(){return{type:"FogExp2",name:this.name,color:this.color.getHex(),density:this.density}}}class ZS extends zn{constructor(){super(),this.isScene=!0,this.type="Scene",this.background=null,this.environment=null,this.fog=null,this.backgroundBlurriness=0,this.backgroundIntensity=1,this.backgroundRotation=new ji,this.environmentIntensity=1,this.environmentRotation=new ji,this.overrideMaterial=null,typeof __THREE_DEVTOOLS__<"u"&&__THREE_DEVTOOLS__.dispatchEvent(new CustomEvent("observe",{detail:this}))}copy(e,i){return super.copy(e,i),e.background!==null&&(this.background=e.background.clone()),e.environment!==null&&(this.environment=e.environment.clone()),e.fog!==null&&(this.fog=e.fog.clone()),this.backgroundBlurriness=e.backgroundBlurriness,this.backgroundIntensity=e.backgroundIntensity,this.backgroundRotation.copy(e.backgroundRotation),this.environmentIntensity=e.environmentIntensity,this.environmentRotation.copy(e.environmentRotation),e.overrideMaterial!==null&&(this.overrideMaterial=e.overrideMaterial.clone()),this.matrixAutoUpdate=e.matrixAutoUpdate,this}toJSON(e){const i=super.toJSON(e);return this.fog!==null&&(i.object.fog=this.fog.toJSON()),this.backgroundBlurriness>0&&(i.object.backgroundBlurriness=this.backgroundBlurriness),this.backgroundIntensity!==1&&(i.object.backgroundIntensity=this.backgroundIntensity),i.object.backgroundRotation=this.backgroundRotation.toArray(),this.environmentIntensity!==1&&(i.object.environmentIntensity=this.environmentIntensity),i.object.environmentRotation=this.environmentRotation.toArray(),i}}const Ri=new K,pa=new K,Ed=new K,ma=new K,Tr=new K,Ar=new K,u_=new K,Td=new K,Ad=new K,Rd=new K,wd=new nn,Cd=new nn,Dd=new nn;class Ci{constructor(e=new K,i=new K,s=new K){this.a=e,this.b=i,this.c=s}static getNormal(e,i,s,l){l.subVectors(s,i),Ri.subVectors(e,i),l.cross(Ri);const c=l.lengthSq();return c>0?l.multiplyScalar(1/Math.sqrt(c)):l.set(0,0,0)}static getBarycoord(e,i,s,l,c){Ri.subVectors(l,i),pa.subVectors(s,i),Ed.subVectors(e,i);const d=Ri.dot(Ri),p=Ri.dot(pa),m=Ri.dot(Ed),h=pa.dot(pa),v=pa.dot(Ed),y=d*h-p*p;if(y===0)return c.set(0,0,0),null;const g=1/y,x=(h*m-p*v)*g,E=(d*v-p*m)*g;return c.set(1-x-E,E,x)}static containsPoint(e,i,s,l){return this.getBarycoord(e,i,s,l,ma)===null?!1:ma.x>=0&&ma.y>=0&&ma.x+ma.y<=1}static getInterpolation(e,i,s,l,c,d,p,m){return this.getBarycoord(e,i,s,l,ma)===null?(m.x=0,m.y=0,"z"in m&&(m.z=0),"w"in m&&(m.w=0),null):(m.setScalar(0),m.addScaledVector(c,ma.x),m.addScaledVector(d,ma.y),m.addScaledVector(p,ma.z),m)}static getInterpolatedAttribute(e,i,s,l,c,d){return wd.setScalar(0),Cd.setScalar(0),Dd.setScalar(0),wd.fromBufferAttribute(e,i),Cd.fromBufferAttribute(e,s),Dd.fromBufferAttribute(e,l),d.setScalar(0),d.addScaledVector(wd,c.x),d.addScaledVector(Cd,c.y),d.addScaledVector(Dd,c.z),d}static isFrontFacing(e,i,s,l){return Ri.subVectors(s,i),pa.subVectors(e,i),Ri.cross(pa).dot(l)<0}set(e,i,s){return this.a.copy(e),this.b.copy(i),this.c.copy(s),this}setFromPointsAndIndices(e,i,s,l){return this.a.copy(e[i]),this.b.copy(e[s]),this.c.copy(e[l]),this}setFromAttributeAndIndices(e,i,s,l){return this.a.fromBufferAttribute(e,i),this.b.fromBufferAttribute(e,s),this.c.fromBufferAttribute(e,l),this}clone(){return new this.constructor().copy(this)}copy(e){return this.a.copy(e.a),this.b.copy(e.b),this.c.copy(e.c),this}getArea(){return Ri.subVectors(this.c,this.b),pa.subVectors(this.a,this.b),Ri.cross(pa).length()*.5}getMidpoint(e){return e.addVectors(this.a,this.b).add(this.c).multiplyScalar(1/3)}getNormal(e){return Ci.getNormal(this.a,this.b,this.c,e)}getPlane(e){return e.setFromCoplanarPoints(this.a,this.b,this.c)}getBarycoord(e,i){return Ci.getBarycoord(e,this.a,this.b,this.c,i)}getInterpolation(e,i,s,l,c){return Ci.getInterpolation(e,this.a,this.b,this.c,i,s,l,c)}containsPoint(e){return Ci.containsPoint(e,this.a,this.b,this.c)}isFrontFacing(e){return Ci.isFrontFacing(this.a,this.b,this.c,e)}intersectsBox(e){return e.intersectsTriangle(this)}closestPointToPoint(e,i){const s=this.a,l=this.b,c=this.c;let d,p;Tr.subVectors(l,s),Ar.subVectors(c,s),Td.subVectors(e,s);const m=Tr.dot(Td),h=Ar.dot(Td);if(m<=0&&h<=0)return i.copy(s);Ad.subVectors(e,l);const v=Tr.dot(Ad),y=Ar.dot(Ad);if(v>=0&&y<=v)return i.copy(l);const g=m*y-v*h;if(g<=0&&m>=0&&v<=0)return d=m/(m-v),i.copy(s).addScaledVector(Tr,d);Rd.subVectors(e,c);const x=Tr.dot(Rd),E=Ar.dot(Rd);if(E>=0&&x<=E)return i.copy(c);const w=x*h-m*E;if(w<=0&&h>=0&&E<=0)return p=h/(h-E),i.copy(s).addScaledVector(Ar,p);const b=v*E-x*y;if(b<=0&&y-v>=0&&x-E>=0)return u_.subVectors(c,l),p=(y-v)/(y-v+(x-E)),i.copy(l).addScaledVector(u_,p);const S=1/(b+w+g);return d=w*S,p=g*S,i.copy(s).addScaledVector(Tr,d).addScaledVector(Ar,p)}equals(e){return e.a.equals(this.a)&&e.b.equals(this.b)&&e.c.equals(this.c)}}class Jo{constructor(e=new K(1/0,1/0,1/0),i=new K(-1/0,-1/0,-1/0)){this.isBox3=!0,this.min=e,this.max=i}set(e,i){return this.min.copy(e),this.max.copy(i),this}setFromArray(e){this.makeEmpty();for(let i=0,s=e.length;i<s;i+=3)this.expandByPoint(wi.fromArray(e,i));return this}setFromBufferAttribute(e){this.makeEmpty();for(let i=0,s=e.count;i<s;i++)this.expandByPoint(wi.fromBufferAttribute(e,i));return this}setFromPoints(e){this.makeEmpty();for(let i=0,s=e.length;i<s;i++)this.expandByPoint(e[i]);return this}setFromCenterAndSize(e,i){const s=wi.copy(i).multiplyScalar(.5);return this.min.copy(e).sub(s),this.max.copy(e).add(s),this}setFromObject(e,i=!1){return this.makeEmpty(),this.expandByObject(e,i)}clone(){return new this.constructor().copy(this)}copy(e){return this.min.copy(e.min),this.max.copy(e.max),this}makeEmpty(){return this.min.x=this.min.y=this.min.z=1/0,this.max.x=this.max.y=this.max.z=-1/0,this}isEmpty(){return this.max.x<this.min.x||this.max.y<this.min.y||this.max.z<this.min.z}getCenter(e){return this.isEmpty()?e.set(0,0,0):e.addVectors(this.min,this.max).multiplyScalar(.5)}getSize(e){return this.isEmpty()?e.set(0,0,0):e.subVectors(this.max,this.min)}expandByPoint(e){return this.min.min(e),this.max.max(e),this}expandByVector(e){return this.min.sub(e),this.max.add(e),this}expandByScalar(e){return this.min.addScalar(-e),this.max.addScalar(e),this}expandByObject(e,i=!1){e.updateWorldMatrix(!1,!1);const s=e.geometry;if(s!==void 0){const c=s.getAttribute("position");if(i===!0&&c!==void 0&&e.isInstancedMesh!==!0)for(let d=0,p=c.count;d<p;d++)e.isMesh===!0?e.getVertexPosition(d,wi):wi.fromBufferAttribute(c,d),wi.applyMatrix4(e.matrixWorld),this.expandByPoint(wi);else e.boundingBox!==void 0?(e.boundingBox===null&&e.computeBoundingBox(),vc.copy(e.boundingBox)):(s.boundingBox===null&&s.computeBoundingBox(),vc.copy(s.boundingBox)),vc.applyMatrix4(e.matrixWorld),this.union(vc)}const l=e.children;for(let c=0,d=l.length;c<d;c++)this.expandByObject(l[c],i);return this}containsPoint(e){return e.x>=this.min.x&&e.x<=this.max.x&&e.y>=this.min.y&&e.y<=this.max.y&&e.z>=this.min.z&&e.z<=this.max.z}containsBox(e){return this.min.x<=e.min.x&&e.max.x<=this.max.x&&this.min.y<=e.min.y&&e.max.y<=this.max.y&&this.min.z<=e.min.z&&e.max.z<=this.max.z}getParameter(e,i){return i.set((e.x-this.min.x)/(this.max.x-this.min.x),(e.y-this.min.y)/(this.max.y-this.min.y),(e.z-this.min.z)/(this.max.z-this.min.z))}intersectsBox(e){return e.max.x>=this.min.x&&e.min.x<=this.max.x&&e.max.y>=this.min.y&&e.min.y<=this.max.y&&e.max.z>=this.min.z&&e.min.z<=this.max.z}intersectsSphere(e){return this.clampPoint(e.center,wi),wi.distanceToSquared(e.center)<=e.radius*e.radius}intersectsPlane(e){let i,s;return e.normal.x>0?(i=e.normal.x*this.min.x,s=e.normal.x*this.max.x):(i=e.normal.x*this.max.x,s=e.normal.x*this.min.x),e.normal.y>0?(i+=e.normal.y*this.min.y,s+=e.normal.y*this.max.y):(i+=e.normal.y*this.max.y,s+=e.normal.y*this.min.y),e.normal.z>0?(i+=e.normal.z*this.min.z,s+=e.normal.z*this.max.z):(i+=e.normal.z*this.max.z,s+=e.normal.z*this.min.z),i<=-e.constant&&s>=-e.constant}intersectsTriangle(e){if(this.isEmpty())return!1;this.getCenter(Ho),xc.subVectors(this.max,Ho),Rr.subVectors(e.a,Ho),wr.subVectors(e.b,Ho),Cr.subVectors(e.c,Ho),Ja.subVectors(wr,Rr),$a.subVectors(Cr,wr),Ts.subVectors(Rr,Cr);let i=[0,-Ja.z,Ja.y,0,-$a.z,$a.y,0,-Ts.z,Ts.y,Ja.z,0,-Ja.x,$a.z,0,-$a.x,Ts.z,0,-Ts.x,-Ja.y,Ja.x,0,-$a.y,$a.x,0,-Ts.y,Ts.x,0];return!Nd(i,Rr,wr,Cr,xc)||(i=[1,0,0,0,1,0,0,0,1],!Nd(i,Rr,wr,Cr,xc))?!1:(yc.crossVectors(Ja,$a),i=[yc.x,yc.y,yc.z],Nd(i,Rr,wr,Cr,xc))}clampPoint(e,i){return i.copy(e).clamp(this.min,this.max)}distanceToPoint(e){return this.clampPoint(e,wi).distanceTo(e)}getBoundingSphere(e){return this.isEmpty()?e.makeEmpty():(this.getCenter(e.center),e.radius=this.getSize(wi).length()*.5),e}intersect(e){return this.min.max(e.min),this.max.min(e.max),this.isEmpty()&&this.makeEmpty(),this}union(e){return this.min.min(e.min),this.max.max(e.max),this}applyMatrix4(e){return this.isEmpty()?this:(ga[0].set(this.min.x,this.min.y,this.min.z).applyMatrix4(e),ga[1].set(this.min.x,this.min.y,this.max.z).applyMatrix4(e),ga[2].set(this.min.x,this.max.y,this.min.z).applyMatrix4(e),ga[3].set(this.min.x,this.max.y,this.max.z).applyMatrix4(e),ga[4].set(this.max.x,this.min.y,this.min.z).applyMatrix4(e),ga[5].set(this.max.x,this.min.y,this.max.z).applyMatrix4(e),ga[6].set(this.max.x,this.max.y,this.min.z).applyMatrix4(e),ga[7].set(this.max.x,this.max.y,this.max.z).applyMatrix4(e),this.setFromPoints(ga),this)}translate(e){return this.min.add(e),this.max.add(e),this}equals(e){return e.min.equals(this.min)&&e.max.equals(this.max)}toJSON(){return{min:this.min.toArray(),max:this.max.toArray()}}fromJSON(e){return this.min.fromArray(e.min),this.max.fromArray(e.max),this}}const ga=[new K,new K,new K,new K,new K,new K,new K,new K],wi=new K,vc=new Jo,Rr=new K,wr=new K,Cr=new K,Ja=new K,$a=new K,Ts=new K,Ho=new K,xc=new K,yc=new K,As=new K;function Nd(o,e,i,s,l){for(let c=0,d=o.length-3;c<=d;c+=3){As.fromArray(o,c);const p=l.x*Math.abs(As.x)+l.y*Math.abs(As.y)+l.z*Math.abs(As.z),m=e.dot(As),h=i.dot(As),v=s.dot(As);if(Math.max(-Math.max(m,h,v),Math.min(m,h,v))>p)return!1}return!0}const hn=new K,Sc=new ct;let KS=0;class Ni{constructor(e,i,s=!1){if(Array.isArray(e))throw new TypeError("THREE.BufferAttribute: array should be a Typed Array.");this.isBufferAttribute=!0,Object.defineProperty(this,"id",{value:KS++}),this.name="",this.array=e,this.itemSize=i,this.count=e!==void 0?e.length/i:0,this.normalized=s,this.usage=K0,this.updateRanges=[],this.gpuType=Hi,this.version=0}onUploadCallback(){}set needsUpdate(e){e===!0&&this.version++}setUsage(e){return this.usage=e,this}addUpdateRange(e,i){this.updateRanges.push({start:e,count:i})}clearUpdateRanges(){this.updateRanges.length=0}copy(e){return this.name=e.name,this.array=new e.array.constructor(e.array),this.itemSize=e.itemSize,this.count=e.count,this.normalized=e.normalized,this.usage=e.usage,this.gpuType=e.gpuType,this}copyAt(e,i,s){e*=this.itemSize,s*=i.itemSize;for(let l=0,c=this.itemSize;l<c;l++)this.array[e+l]=i.array[s+l];return this}copyArray(e){return this.array.set(e),this}applyMatrix3(e){if(this.itemSize===2)for(let i=0,s=this.count;i<s;i++)Sc.fromBufferAttribute(this,i),Sc.applyMatrix3(e),this.setXY(i,Sc.x,Sc.y);else if(this.itemSize===3)for(let i=0,s=this.count;i<s;i++)hn.fromBufferAttribute(this,i),hn.applyMatrix3(e),this.setXYZ(i,hn.x,hn.y,hn.z);return this}applyMatrix4(e){for(let i=0,s=this.count;i<s;i++)hn.fromBufferAttribute(this,i),hn.applyMatrix4(e),this.setXYZ(i,hn.x,hn.y,hn.z);return this}applyNormalMatrix(e){for(let i=0,s=this.count;i<s;i++)hn.fromBufferAttribute(this,i),hn.applyNormalMatrix(e),this.setXYZ(i,hn.x,hn.y,hn.z);return this}transformDirection(e){for(let i=0,s=this.count;i<s;i++)hn.fromBufferAttribute(this,i),hn.transformDirection(e),this.setXYZ(i,hn.x,hn.y,hn.z);return this}set(e,i=0){return this.array.set(e,i),this}getComponent(e,i){let s=this.array[e*this.itemSize+i];return this.normalized&&(s=zo(s,this.array)),s}setComponent(e,i,s){return this.normalized&&(s=jn(s,this.array)),this.array[e*this.itemSize+i]=s,this}getX(e){let i=this.array[e*this.itemSize];return this.normalized&&(i=zo(i,this.array)),i}setX(e,i){return this.normalized&&(i=jn(i,this.array)),this.array[e*this.itemSize]=i,this}getY(e){let i=this.array[e*this.itemSize+1];return this.normalized&&(i=zo(i,this.array)),i}setY(e,i){return this.normalized&&(i=jn(i,this.array)),this.array[e*this.itemSize+1]=i,this}getZ(e){let i=this.array[e*this.itemSize+2];return this.normalized&&(i=zo(i,this.array)),i}setZ(e,i){return this.normalized&&(i=jn(i,this.array)),this.array[e*this.itemSize+2]=i,this}getW(e){let i=this.array[e*this.itemSize+3];return this.normalized&&(i=zo(i,this.array)),i}setW(e,i){return this.normalized&&(i=jn(i,this.array)),this.array[e*this.itemSize+3]=i,this}setXY(e,i,s){return e*=this.itemSize,this.normalized&&(i=jn(i,this.array),s=jn(s,this.array)),this.array[e+0]=i,this.array[e+1]=s,this}setXYZ(e,i,s,l){return e*=this.itemSize,this.normalized&&(i=jn(i,this.array),s=jn(s,this.array),l=jn(l,this.array)),this.array[e+0]=i,this.array[e+1]=s,this.array[e+2]=l,this}setXYZW(e,i,s,l,c){return e*=this.itemSize,this.normalized&&(i=jn(i,this.array),s=jn(s,this.array),l=jn(l,this.array),c=jn(c,this.array)),this.array[e+0]=i,this.array[e+1]=s,this.array[e+2]=l,this.array[e+3]=c,this}onUpload(e){return this.onUploadCallback=e,this}clone(){return new this.constructor(this.array,this.itemSize).copy(this)}toJSON(){const e={itemSize:this.itemSize,type:this.array.constructor.name,array:Array.from(this.array),normalized:this.normalized};return this.name!==""&&(e.name=this.name),this.usage!==K0&&(e.usage=this.usage),e}}class vv extends Ni{constructor(e,i,s){super(new Uint16Array(e),i,s)}}class xv extends Ni{constructor(e,i,s){super(new Uint32Array(e),i,s)}}class _i extends Ni{constructor(e,i,s){super(new Float32Array(e),i,s)}}const QS=new Jo,Go=new K,Ud=new K;class $c{constructor(e=new K,i=-1){this.isSphere=!0,this.center=e,this.radius=i}set(e,i){return this.center.copy(e),this.radius=i,this}setFromPoints(e,i){const s=this.center;i!==void 0?s.copy(i):QS.setFromPoints(e).getCenter(s);let l=0;for(let c=0,d=e.length;c<d;c++)l=Math.max(l,s.distanceToSquared(e[c]));return this.radius=Math.sqrt(l),this}copy(e){return this.center.copy(e.center),this.radius=e.radius,this}isEmpty(){return this.radius<0}makeEmpty(){return this.center.set(0,0,0),this.radius=-1,this}containsPoint(e){return e.distanceToSquared(this.center)<=this.radius*this.radius}distanceToPoint(e){return e.distanceTo(this.center)-this.radius}intersectsSphere(e){const i=this.radius+e.radius;return e.center.distanceToSquared(this.center)<=i*i}intersectsBox(e){return e.intersectsSphere(this)}intersectsPlane(e){return Math.abs(e.distanceToPoint(this.center))<=this.radius}clampPoint(e,i){const s=this.center.distanceToSquared(e);return i.copy(e),s>this.radius*this.radius&&(i.sub(this.center).normalize(),i.multiplyScalar(this.radius).add(this.center)),i}getBoundingBox(e){return this.isEmpty()?(e.makeEmpty(),e):(e.set(this.center,this.center),e.expandByScalar(this.radius),e)}applyMatrix4(e){return this.center.applyMatrix4(e),this.radius=this.radius*e.getMaxScaleOnAxis(),this}translate(e){return this.center.add(e),this}expandByPoint(e){if(this.isEmpty())return this.center.copy(e),this.radius=0,this;Go.subVectors(e,this.center);const i=Go.lengthSq();if(i>this.radius*this.radius){const s=Math.sqrt(i),l=(s-this.radius)*.5;this.center.addScaledVector(Go,l/s),this.radius+=l}return this}union(e){return e.isEmpty()?this:this.isEmpty()?(this.copy(e),this):(this.center.equals(e.center)===!0?this.radius=Math.max(this.radius,e.radius):(Ud.subVectors(e.center,this.center).setLength(e.radius),this.expandByPoint(Go.copy(e.center).add(Ud)),this.expandByPoint(Go.copy(e.center).sub(Ud))),this)}equals(e){return e.center.equals(this.center)&&e.radius===this.radius}clone(){return new this.constructor().copy(this)}toJSON(){return{radius:this.radius,center:this.center.toArray()}}fromJSON(e){return this.radius=e.radius,this.center.fromArray(e.center),this}}let JS=0;const mi=new Jt,Ld=new zn,Dr=new K,ai=new Jo,Vo=new Jo,Sn=new K;class vi extends Fs{constructor(){super(),this.isBufferGeometry=!0,Object.defineProperty(this,"id",{value:JS++}),this.uuid=Qo(),this.name="",this.type="BufferGeometry",this.index=null,this.indirect=null,this.indirectOffset=0,this.attributes={},this.morphAttributes={},this.morphTargetsRelative=!1,this.groups=[],this.boundingBox=null,this.boundingSphere=null,this.drawRange={start:0,count:1/0},this.userData={}}getIndex(){return this.index}setIndex(e){return Array.isArray(e)?this.index=new(DS(e)?xv:vv)(e,1):this.index=e,this}setIndirect(e,i=0){return this.indirect=e,this.indirectOffset=i,this}getIndirect(){return this.indirect}getAttribute(e){return this.attributes[e]}setAttribute(e,i){return this.attributes[e]=i,this}deleteAttribute(e){return delete this.attributes[e],this}hasAttribute(e){return this.attributes[e]!==void 0}addGroup(e,i,s=0){this.groups.push({start:e,count:i,materialIndex:s})}clearGroups(){this.groups=[]}setDrawRange(e,i){this.drawRange.start=e,this.drawRange.count=i}applyMatrix4(e){const i=this.attributes.position;i!==void 0&&(i.applyMatrix4(e),i.needsUpdate=!0);const s=this.attributes.normal;if(s!==void 0){const c=new ht().getNormalMatrix(e);s.applyNormalMatrix(c),s.needsUpdate=!0}const l=this.attributes.tangent;return l!==void 0&&(l.transformDirection(e),l.needsUpdate=!0),this.boundingBox!==null&&this.computeBoundingBox(),this.boundingSphere!==null&&this.computeBoundingSphere(),this}applyQuaternion(e){return mi.makeRotationFromQuaternion(e),this.applyMatrix4(mi),this}rotateX(e){return mi.makeRotationX(e),this.applyMatrix4(mi),this}rotateY(e){return mi.makeRotationY(e),this.applyMatrix4(mi),this}rotateZ(e){return mi.makeRotationZ(e),this.applyMatrix4(mi),this}translate(e,i,s){return mi.makeTranslation(e,i,s),this.applyMatrix4(mi),this}scale(e,i,s){return mi.makeScale(e,i,s),this.applyMatrix4(mi),this}lookAt(e){return Ld.lookAt(e),Ld.updateMatrix(),this.applyMatrix4(Ld.matrix),this}center(){return this.computeBoundingBox(),this.boundingBox.getCenter(Dr).negate(),this.translate(Dr.x,Dr.y,Dr.z),this}setFromPoints(e){const i=this.getAttribute("position");if(i===void 0){const s=[];for(let l=0,c=e.length;l<c;l++){const d=e[l];s.push(d.x,d.y,d.z||0)}this.setAttribute("position",new _i(s,3))}else{const s=Math.min(e.length,i.count);for(let l=0;l<s;l++){const c=e[l];i.setXYZ(l,c.x,c.y,c.z||0)}e.length>i.count&&at("BufferGeometry: Buffer size too small for points data. Use .dispose() and create a new geometry."),i.needsUpdate=!0}return this}computeBoundingBox(){this.boundingBox===null&&(this.boundingBox=new Jo);const e=this.attributes.position,i=this.morphAttributes.position;if(e&&e.isGLBufferAttribute){Dt("BufferGeometry.computeBoundingBox(): GLBufferAttribute requires a manual bounding box.",this),this.boundingBox.set(new K(-1/0,-1/0,-1/0),new K(1/0,1/0,1/0));return}if(e!==void 0){if(this.boundingBox.setFromBufferAttribute(e),i)for(let s=0,l=i.length;s<l;s++){const c=i[s];ai.setFromBufferAttribute(c),this.morphTargetsRelative?(Sn.addVectors(this.boundingBox.min,ai.min),this.boundingBox.expandByPoint(Sn),Sn.addVectors(this.boundingBox.max,ai.max),this.boundingBox.expandByPoint(Sn)):(this.boundingBox.expandByPoint(ai.min),this.boundingBox.expandByPoint(ai.max))}}else this.boundingBox.makeEmpty();(isNaN(this.boundingBox.min.x)||isNaN(this.boundingBox.min.y)||isNaN(this.boundingBox.min.z))&&Dt('BufferGeometry.computeBoundingBox(): Computed min/max have NaN values. The "position" attribute is likely to have NaN values.',this)}computeBoundingSphere(){this.boundingSphere===null&&(this.boundingSphere=new $c);const e=this.attributes.position,i=this.morphAttributes.position;if(e&&e.isGLBufferAttribute){Dt("BufferGeometry.computeBoundingSphere(): GLBufferAttribute requires a manual bounding sphere.",this),this.boundingSphere.set(new K,1/0);return}if(e){const s=this.boundingSphere.center;if(ai.setFromBufferAttribute(e),i)for(let c=0,d=i.length;c<d;c++){const p=i[c];Vo.setFromBufferAttribute(p),this.morphTargetsRelative?(Sn.addVectors(ai.min,Vo.min),ai.expandByPoint(Sn),Sn.addVectors(ai.max,Vo.max),ai.expandByPoint(Sn)):(ai.expandByPoint(Vo.min),ai.expandByPoint(Vo.max))}ai.getCenter(s);let l=0;for(let c=0,d=e.count;c<d;c++)Sn.fromBufferAttribute(e,c),l=Math.max(l,s.distanceToSquared(Sn));if(i)for(let c=0,d=i.length;c<d;c++){const p=i[c],m=this.morphTargetsRelative;for(let h=0,v=p.count;h<v;h++)Sn.fromBufferAttribute(p,h),m&&(Dr.fromBufferAttribute(e,h),Sn.add(Dr)),l=Math.max(l,s.distanceToSquared(Sn))}this.boundingSphere.radius=Math.sqrt(l),isNaN(this.boundingSphere.radius)&&Dt('BufferGeometry.computeBoundingSphere(): Computed radius is NaN. The "position" attribute is likely to have NaN values.',this)}}computeTangents(){const e=this.index,i=this.attributes;if(e===null||i.position===void 0||i.normal===void 0||i.uv===void 0){Dt("BufferGeometry: .computeTangents() failed. Missing required attributes (index, position, normal or uv)");return}const s=i.position,l=i.normal,c=i.uv;this.hasAttribute("tangent")===!1&&this.setAttribute("tangent",new Ni(new Float32Array(4*s.count),4));const d=this.getAttribute("tangent"),p=[],m=[];for(let T=0;T<s.count;T++)p[T]=new K,m[T]=new K;const h=new K,v=new K,y=new K,g=new ct,x=new ct,E=new ct,w=new K,b=new K;function S(T,D,le){h.fromBufferAttribute(s,T),v.fromBufferAttribute(s,D),y.fromBufferAttribute(s,le),g.fromBufferAttribute(c,T),x.fromBufferAttribute(c,D),E.fromBufferAttribute(c,le),v.sub(h),y.sub(h),x.sub(g),E.sub(g);const G=1/(x.x*E.y-E.x*x.y);isFinite(G)&&(w.copy(v).multiplyScalar(E.y).addScaledVector(y,-x.y).multiplyScalar(G),b.copy(y).multiplyScalar(x.x).addScaledVector(v,-E.x).multiplyScalar(G),p[T].add(w),p[D].add(w),p[le].add(w),m[T].add(b),m[D].add(b),m[le].add(b))}let C=this.groups;C.length===0&&(C=[{start:0,count:e.count}]);for(let T=0,D=C.length;T<D;++T){const le=C[T],G=le.start,te=le.count;for(let se=G,ue=G+te;se<ue;se+=3)S(e.getX(se+0),e.getX(se+1),e.getX(se+2))}const U=new K,N=new K,V=new K,H=new K;function F(T){V.fromBufferAttribute(l,T),H.copy(V);const D=p[T];U.copy(D),U.sub(V.multiplyScalar(V.dot(D))).normalize(),N.crossVectors(H,D);const G=N.dot(m[T])<0?-1:1;d.setXYZW(T,U.x,U.y,U.z,G)}for(let T=0,D=C.length;T<D;++T){const le=C[T],G=le.start,te=le.count;for(let se=G,ue=G+te;se<ue;se+=3)F(e.getX(se+0)),F(e.getX(se+1)),F(e.getX(se+2))}}computeVertexNormals(){const e=this.index,i=this.getAttribute("position");if(i!==void 0){let s=this.getAttribute("normal");if(s===void 0)s=new Ni(new Float32Array(i.count*3),3),this.setAttribute("normal",s);else for(let g=0,x=s.count;g<x;g++)s.setXYZ(g,0,0,0);const l=new K,c=new K,d=new K,p=new K,m=new K,h=new K,v=new K,y=new K;if(e)for(let g=0,x=e.count;g<x;g+=3){const E=e.getX(g+0),w=e.getX(g+1),b=e.getX(g+2);l.fromBufferAttribute(i,E),c.fromBufferAttribute(i,w),d.fromBufferAttribute(i,b),v.subVectors(d,c),y.subVectors(l,c),v.cross(y),p.fromBufferAttribute(s,E),m.fromBufferAttribute(s,w),h.fromBufferAttribute(s,b),p.add(v),m.add(v),h.add(v),s.setXYZ(E,p.x,p.y,p.z),s.setXYZ(w,m.x,m.y,m.z),s.setXYZ(b,h.x,h.y,h.z)}else for(let g=0,x=i.count;g<x;g+=3)l.fromBufferAttribute(i,g+0),c.fromBufferAttribute(i,g+1),d.fromBufferAttribute(i,g+2),v.subVectors(d,c),y.subVectors(l,c),v.cross(y),s.setXYZ(g+0,v.x,v.y,v.z),s.setXYZ(g+1,v.x,v.y,v.z),s.setXYZ(g+2,v.x,v.y,v.z);this.normalizeNormals(),s.needsUpdate=!0}}normalizeNormals(){const e=this.attributes.normal;for(let i=0,s=e.count;i<s;i++)Sn.fromBufferAttribute(e,i),Sn.normalize(),e.setXYZ(i,Sn.x,Sn.y,Sn.z)}toNonIndexed(){function e(p,m){const h=p.array,v=p.itemSize,y=p.normalized,g=new h.constructor(m.length*v);let x=0,E=0;for(let w=0,b=m.length;w<b;w++){p.isInterleavedBufferAttribute?x=m[w]*p.data.stride+p.offset:x=m[w]*v;for(let S=0;S<v;S++)g[E++]=h[x++]}return new Ni(g,v,y)}if(this.index===null)return at("BufferGeometry.toNonIndexed(): BufferGeometry is already non-indexed."),this;const i=new vi,s=this.index.array,l=this.attributes;for(const p in l){const m=l[p],h=e(m,s);i.setAttribute(p,h)}const c=this.morphAttributes;for(const p in c){const m=[],h=c[p];for(let v=0,y=h.length;v<y;v++){const g=h[v],x=e(g,s);m.push(x)}i.morphAttributes[p]=m}i.morphTargetsRelative=this.morphTargetsRelative;const d=this.groups;for(let p=0,m=d.length;p<m;p++){const h=d[p];i.addGroup(h.start,h.count,h.materialIndex)}return i}toJSON(){const e={metadata:{version:4.7,type:"BufferGeometry",generator:"BufferGeometry.toJSON"}};if(e.uuid=this.uuid,e.type=this.type,this.name!==""&&(e.name=this.name),Object.keys(this.userData).length>0&&(e.userData=this.userData),this.parameters!==void 0){const m=this.parameters;for(const h in m)m[h]!==void 0&&(e[h]=m[h]);return e}e.data={attributes:{}};const i=this.index;i!==null&&(e.data.index={type:i.array.constructor.name,array:Array.prototype.slice.call(i.array)});const s=this.attributes;for(const m in s){const h=s[m];e.data.attributes[m]=h.toJSON(e.data)}const l={};let c=!1;for(const m in this.morphAttributes){const h=this.morphAttributes[m],v=[];for(let y=0,g=h.length;y<g;y++){const x=h[y];v.push(x.toJSON(e.data))}v.length>0&&(l[m]=v,c=!0)}c&&(e.data.morphAttributes=l,e.data.morphTargetsRelative=this.morphTargetsRelative);const d=this.groups;d.length>0&&(e.data.groups=JSON.parse(JSON.stringify(d)));const p=this.boundingSphere;return p!==null&&(e.data.boundingSphere=p.toJSON()),e}clone(){return new this.constructor().copy(this)}copy(e){this.index=null,this.attributes={},this.morphAttributes={},this.groups=[],this.boundingBox=null,this.boundingSphere=null;const i={};this.name=e.name;const s=e.index;s!==null&&this.setIndex(s.clone());const l=e.attributes;for(const h in l){const v=l[h];this.setAttribute(h,v.clone(i))}const c=e.morphAttributes;for(const h in c){const v=[],y=c[h];for(let g=0,x=y.length;g<x;g++)v.push(y[g].clone(i));this.morphAttributes[h]=v}this.morphTargetsRelative=e.morphTargetsRelative;const d=e.groups;for(let h=0,v=d.length;h<v;h++){const y=d[h];this.addGroup(y.start,y.count,y.materialIndex)}const p=e.boundingBox;p!==null&&(this.boundingBox=p.clone());const m=e.boundingSphere;return m!==null&&(this.boundingSphere=m.clone()),this.drawRange.start=e.drawRange.start,this.drawRange.count=e.drawRange.count,this.userData=e.userData,this}dispose(){this.dispatchEvent({type:"dispose"})}}let $S=0;class Xr extends Fs{constructor(){super(),this.isMaterial=!0,Object.defineProperty(this,"id",{value:$S++}),this.uuid=Qo(),this.name="",this.type="Material",this.blending=Fr,this.side=ss,this.vertexColors=!1,this.opacity=1,this.transparent=!1,this.alphaHash=!1,this.blendSrc=Yd,this.blendDst=Zd,this.blendEquation=Us,this.blendSrcAlpha=null,this.blendDstAlpha=null,this.blendEquationAlpha=null,this.blendColor=new At(0,0,0),this.blendAlpha=0,this.depthFunc=Br,this.depthTest=!0,this.depthWrite=!0,this.stencilWriteMask=255,this.stencilFunc=Z0,this.stencilRef=0,this.stencilFuncMask=255,this.stencilFail=yr,this.stencilZFail=yr,this.stencilZPass=yr,this.stencilWrite=!1,this.clippingPlanes=null,this.clipIntersection=!1,this.clipShadows=!1,this.shadowSide=null,this.colorWrite=!0,this.precision=null,this.polygonOffset=!1,this.polygonOffsetFactor=0,this.polygonOffsetUnits=0,this.dithering=!1,this.alphaToCoverage=!1,this.premultipliedAlpha=!1,this.forceSinglePass=!1,this.allowOverride=!0,this.visible=!0,this.toneMapped=!0,this.userData={},this.version=0,this._alphaTest=0}get alphaTest(){return this._alphaTest}set alphaTest(e){this._alphaTest>0!=e>0&&this.version++,this._alphaTest=e}onBeforeRender(){}onBeforeCompile(){}customProgramCacheKey(){return this.onBeforeCompile.toString()}setValues(e){if(e!==void 0)for(const i in e){const s=e[i];if(s===void 0){at(`Material: parameter '${i}' has value of undefined.`);continue}const l=this[i];if(l===void 0){at(`Material: '${i}' is not a property of THREE.${this.type}.`);continue}l&&l.isColor?l.set(s):l&&l.isVector3&&s&&s.isVector3?l.copy(s):this[i]=s}}toJSON(e){const i=e===void 0||typeof e=="string";i&&(e={textures:{},images:{}});const s={metadata:{version:4.7,type:"Material",generator:"Material.toJSON"}};s.uuid=this.uuid,s.type=this.type,this.name!==""&&(s.name=this.name),this.color&&this.color.isColor&&(s.color=this.color.getHex()),this.roughness!==void 0&&(s.roughness=this.roughness),this.metalness!==void 0&&(s.metalness=this.metalness),this.sheen!==void 0&&(s.sheen=this.sheen),this.sheenColor&&this.sheenColor.isColor&&(s.sheenColor=this.sheenColor.getHex()),this.sheenRoughness!==void 0&&(s.sheenRoughness=this.sheenRoughness),this.emissive&&this.emissive.isColor&&(s.emissive=this.emissive.getHex()),this.emissiveIntensity!==void 0&&this.emissiveIntensity!==1&&(s.emissiveIntensity=this.emissiveIntensity),this.specular&&this.specular.isColor&&(s.specular=this.specular.getHex()),this.specularIntensity!==void 0&&(s.specularIntensity=this.specularIntensity),this.specularColor&&this.specularColor.isColor&&(s.specularColor=this.specularColor.getHex()),this.shininess!==void 0&&(s.shininess=this.shininess),this.clearcoat!==void 0&&(s.clearcoat=this.clearcoat),this.clearcoatRoughness!==void 0&&(s.clearcoatRoughness=this.clearcoatRoughness),this.clearcoatMap&&this.clearcoatMap.isTexture&&(s.clearcoatMap=this.clearcoatMap.toJSON(e).uuid),this.clearcoatRoughnessMap&&this.clearcoatRoughnessMap.isTexture&&(s.clearcoatRoughnessMap=this.clearcoatRoughnessMap.toJSON(e).uuid),this.clearcoatNormalMap&&this.clearcoatNormalMap.isTexture&&(s.clearcoatNormalMap=this.clearcoatNormalMap.toJSON(e).uuid,s.clearcoatNormalScale=this.clearcoatNormalScale.toArray()),this.sheenColorMap&&this.sheenColorMap.isTexture&&(s.sheenColorMap=this.sheenColorMap.toJSON(e).uuid),this.sheenRoughnessMap&&this.sheenRoughnessMap.isTexture&&(s.sheenRoughnessMap=this.sheenRoughnessMap.toJSON(e).uuid),this.dispersion!==void 0&&(s.dispersion=this.dispersion),this.iridescence!==void 0&&(s.iridescence=this.iridescence),this.iridescenceIOR!==void 0&&(s.iridescenceIOR=this.iridescenceIOR),this.iridescenceThicknessRange!==void 0&&(s.iridescenceThicknessRange=this.iridescenceThicknessRange),this.iridescenceMap&&this.iridescenceMap.isTexture&&(s.iridescenceMap=this.iridescenceMap.toJSON(e).uuid),this.iridescenceThicknessMap&&this.iridescenceThicknessMap.isTexture&&(s.iridescenceThicknessMap=this.iridescenceThicknessMap.toJSON(e).uuid),this.anisotropy!==void 0&&(s.anisotropy=this.anisotropy),this.anisotropyRotation!==void 0&&(s.anisotropyRotation=this.anisotropyRotation),this.anisotropyMap&&this.anisotropyMap.isTexture&&(s.anisotropyMap=this.anisotropyMap.toJSON(e).uuid),this.map&&this.map.isTexture&&(s.map=this.map.toJSON(e).uuid),this.matcap&&this.matcap.isTexture&&(s.matcap=this.matcap.toJSON(e).uuid),this.alphaMap&&this.alphaMap.isTexture&&(s.alphaMap=this.alphaMap.toJSON(e).uuid),this.lightMap&&this.lightMap.isTexture&&(s.lightMap=this.lightMap.toJSON(e).uuid,s.lightMapIntensity=this.lightMapIntensity),this.aoMap&&this.aoMap.isTexture&&(s.aoMap=this.aoMap.toJSON(e).uuid,s.aoMapIntensity=this.aoMapIntensity),this.bumpMap&&this.bumpMap.isTexture&&(s.bumpMap=this.bumpMap.toJSON(e).uuid,s.bumpScale=this.bumpScale),this.normalMap&&this.normalMap.isTexture&&(s.normalMap=this.normalMap.toJSON(e).uuid,s.normalMapType=this.normalMapType,s.normalScale=this.normalScale.toArray()),this.displacementMap&&this.displacementMap.isTexture&&(s.displacementMap=this.displacementMap.toJSON(e).uuid,s.displacementScale=this.displacementScale,s.displacementBias=this.displacementBias),this.roughnessMap&&this.roughnessMap.isTexture&&(s.roughnessMap=this.roughnessMap.toJSON(e).uuid),this.metalnessMap&&this.metalnessMap.isTexture&&(s.metalnessMap=this.metalnessMap.toJSON(e).uuid),this.emissiveMap&&this.emissiveMap.isTexture&&(s.emissiveMap=this.emissiveMap.toJSON(e).uuid),this.specularMap&&this.specularMap.isTexture&&(s.specularMap=this.specularMap.toJSON(e).uuid),this.specularIntensityMap&&this.specularIntensityMap.isTexture&&(s.specularIntensityMap=this.specularIntensityMap.toJSON(e).uuid),this.specularColorMap&&this.specularColorMap.isTexture&&(s.specularColorMap=this.specularColorMap.toJSON(e).uuid),this.envMap&&this.envMap.isTexture&&(s.envMap=this.envMap.toJSON(e).uuid,this.combine!==void 0&&(s.combine=this.combine)),this.envMapRotation!==void 0&&(s.envMapRotation=this.envMapRotation.toArray()),this.envMapIntensity!==void 0&&(s.envMapIntensity=this.envMapIntensity),this.reflectivity!==void 0&&(s.reflectivity=this.reflectivity),this.refractionRatio!==void 0&&(s.refractionRatio=this.refractionRatio),this.gradientMap&&this.gradientMap.isTexture&&(s.gradientMap=this.gradientMap.toJSON(e).uuid),this.transmission!==void 0&&(s.transmission=this.transmission),this.transmissionMap&&this.transmissionMap.isTexture&&(s.transmissionMap=this.transmissionMap.toJSON(e).uuid),this.thickness!==void 0&&(s.thickness=this.thickness),this.thicknessMap&&this.thicknessMap.isTexture&&(s.thicknessMap=this.thicknessMap.toJSON(e).uuid),this.attenuationDistance!==void 0&&this.attenuationDistance!==1/0&&(s.attenuationDistance=this.attenuationDistance),this.attenuationColor!==void 0&&(s.attenuationColor=this.attenuationColor.getHex()),this.size!==void 0&&(s.size=this.size),this.shadowSide!==null&&(s.shadowSide=this.shadowSide),this.sizeAttenuation!==void 0&&(s.sizeAttenuation=this.sizeAttenuation),this.blending!==Fr&&(s.blending=this.blending),this.side!==ss&&(s.side=this.side),this.vertexColors===!0&&(s.vertexColors=!0),this.opacity<1&&(s.opacity=this.opacity),this.transparent===!0&&(s.transparent=!0),this.blendSrc!==Yd&&(s.blendSrc=this.blendSrc),this.blendDst!==Zd&&(s.blendDst=this.blendDst),this.blendEquation!==Us&&(s.blendEquation=this.blendEquation),this.blendSrcAlpha!==null&&(s.blendSrcAlpha=this.blendSrcAlpha),this.blendDstAlpha!==null&&(s.blendDstAlpha=this.blendDstAlpha),this.blendEquationAlpha!==null&&(s.blendEquationAlpha=this.blendEquationAlpha),this.blendColor&&this.blendColor.isColor&&(s.blendColor=this.blendColor.getHex()),this.blendAlpha!==0&&(s.blendAlpha=this.blendAlpha),this.depthFunc!==Br&&(s.depthFunc=this.depthFunc),this.depthTest===!1&&(s.depthTest=this.depthTest),this.depthWrite===!1&&(s.depthWrite=this.depthWrite),this.colorWrite===!1&&(s.colorWrite=this.colorWrite),this.stencilWriteMask!==255&&(s.stencilWriteMask=this.stencilWriteMask),this.stencilFunc!==Z0&&(s.stencilFunc=this.stencilFunc),this.stencilRef!==0&&(s.stencilRef=this.stencilRef),this.stencilFuncMask!==255&&(s.stencilFuncMask=this.stencilFuncMask),this.stencilFail!==yr&&(s.stencilFail=this.stencilFail),this.stencilZFail!==yr&&(s.stencilZFail=this.stencilZFail),this.stencilZPass!==yr&&(s.stencilZPass=this.stencilZPass),this.stencilWrite===!0&&(s.stencilWrite=this.stencilWrite),this.rotation!==void 0&&this.rotation!==0&&(s.rotation=this.rotation),this.polygonOffset===!0&&(s.polygonOffset=!0),this.polygonOffsetFactor!==0&&(s.polygonOffsetFactor=this.polygonOffsetFactor),this.polygonOffsetUnits!==0&&(s.polygonOffsetUnits=this.polygonOffsetUnits),this.linewidth!==void 0&&this.linewidth!==1&&(s.linewidth=this.linewidth),this.dashSize!==void 0&&(s.dashSize=this.dashSize),this.gapSize!==void 0&&(s.gapSize=this.gapSize),this.scale!==void 0&&(s.scale=this.scale),this.dithering===!0&&(s.dithering=!0),this.alphaTest>0&&(s.alphaTest=this.alphaTest),this.alphaHash===!0&&(s.alphaHash=!0),this.alphaToCoverage===!0&&(s.alphaToCoverage=!0),this.premultipliedAlpha===!0&&(s.premultipliedAlpha=!0),this.forceSinglePass===!0&&(s.forceSinglePass=!0),this.allowOverride===!1&&(s.allowOverride=!1),this.wireframe===!0&&(s.wireframe=!0),this.wireframeLinewidth>1&&(s.wireframeLinewidth=this.wireframeLinewidth),this.wireframeLinecap!=="round"&&(s.wireframeLinecap=this.wireframeLinecap),this.wireframeLinejoin!=="round"&&(s.wireframeLinejoin=this.wireframeLinejoin),this.flatShading===!0&&(s.flatShading=!0),this.visible===!1&&(s.visible=!1),this.toneMapped===!1&&(s.toneMapped=!1),this.fog===!1&&(s.fog=!1),Object.keys(this.userData).length>0&&(s.userData=this.userData);function l(c){const d=[];for(const p in c){const m=c[p];delete m.metadata,d.push(m)}return d}if(i){const c=l(e.textures),d=l(e.images);c.length>0&&(s.textures=c),d.length>0&&(s.images=d)}return s}clone(){return new this.constructor().copy(this)}copy(e){this.name=e.name,this.blending=e.blending,this.side=e.side,this.vertexColors=e.vertexColors,this.opacity=e.opacity,this.transparent=e.transparent,this.blendSrc=e.blendSrc,this.blendDst=e.blendDst,this.blendEquation=e.blendEquation,this.blendSrcAlpha=e.blendSrcAlpha,this.blendDstAlpha=e.blendDstAlpha,this.blendEquationAlpha=e.blendEquationAlpha,this.blendColor.copy(e.blendColor),this.blendAlpha=e.blendAlpha,this.depthFunc=e.depthFunc,this.depthTest=e.depthTest,this.depthWrite=e.depthWrite,this.stencilWriteMask=e.stencilWriteMask,this.stencilFunc=e.stencilFunc,this.stencilRef=e.stencilRef,this.stencilFuncMask=e.stencilFuncMask,this.stencilFail=e.stencilFail,this.stencilZFail=e.stencilZFail,this.stencilZPass=e.stencilZPass,this.stencilWrite=e.stencilWrite;const i=e.clippingPlanes;let s=null;if(i!==null){const l=i.length;s=new Array(l);for(let c=0;c!==l;++c)s[c]=i[c].clone()}return this.clippingPlanes=s,this.clipIntersection=e.clipIntersection,this.clipShadows=e.clipShadows,this.shadowSide=e.shadowSide,this.colorWrite=e.colorWrite,this.precision=e.precision,this.polygonOffset=e.polygonOffset,this.polygonOffsetFactor=e.polygonOffsetFactor,this.polygonOffsetUnits=e.polygonOffsetUnits,this.dithering=e.dithering,this.alphaTest=e.alphaTest,this.alphaHash=e.alphaHash,this.alphaToCoverage=e.alphaToCoverage,this.premultipliedAlpha=e.premultipliedAlpha,this.forceSinglePass=e.forceSinglePass,this.allowOverride=e.allowOverride,this.visible=e.visible,this.toneMapped=e.toneMapped,this.userData=JSON.parse(JSON.stringify(e.userData)),this}dispose(){this.dispatchEvent({type:"dispose"})}set needsUpdate(e){e===!0&&this.version++}}const _a=new K,Od=new K,bc=new K,es=new K,Pd=new K,Mc=new K,Id=new K;class ep{constructor(e=new K,i=new K(0,0,-1)){this.origin=e,this.direction=i}set(e,i){return this.origin.copy(e),this.direction.copy(i),this}copy(e){return this.origin.copy(e.origin),this.direction.copy(e.direction),this}at(e,i){return i.copy(this.origin).addScaledVector(this.direction,e)}lookAt(e){return this.direction.copy(e).sub(this.origin).normalize(),this}recast(e){return this.origin.copy(this.at(e,_a)),this}closestPointToPoint(e,i){i.subVectors(e,this.origin);const s=i.dot(this.direction);return s<0?i.copy(this.origin):i.copy(this.origin).addScaledVector(this.direction,s)}distanceToPoint(e){return Math.sqrt(this.distanceSqToPoint(e))}distanceSqToPoint(e){const i=_a.subVectors(e,this.origin).dot(this.direction);return i<0?this.origin.distanceToSquared(e):(_a.copy(this.origin).addScaledVector(this.direction,i),_a.distanceToSquared(e))}distanceSqToSegment(e,i,s,l){Od.copy(e).add(i).multiplyScalar(.5),bc.copy(i).sub(e).normalize(),es.copy(this.origin).sub(Od);const c=e.distanceTo(i)*.5,d=-this.direction.dot(bc),p=es.dot(this.direction),m=-es.dot(bc),h=es.lengthSq(),v=Math.abs(1-d*d);let y,g,x,E;if(v>0)if(y=d*m-p,g=d*p-m,E=c*v,y>=0)if(g>=-E)if(g<=E){const w=1/v;y*=w,g*=w,x=y*(y+d*g+2*p)+g*(d*y+g+2*m)+h}else g=c,y=Math.max(0,-(d*g+p)),x=-y*y+g*(g+2*m)+h;else g=-c,y=Math.max(0,-(d*g+p)),x=-y*y+g*(g+2*m)+h;else g<=-E?(y=Math.max(0,-(-d*c+p)),g=y>0?-c:Math.min(Math.max(-c,-m),c),x=-y*y+g*(g+2*m)+h):g<=E?(y=0,g=Math.min(Math.max(-c,-m),c),x=g*(g+2*m)+h):(y=Math.max(0,-(d*c+p)),g=y>0?c:Math.min(Math.max(-c,-m),c),x=-y*y+g*(g+2*m)+h);else g=d>0?-c:c,y=Math.max(0,-(d*g+p)),x=-y*y+g*(g+2*m)+h;return s&&s.copy(this.origin).addScaledVector(this.direction,y),l&&l.copy(Od).addScaledVector(bc,g),x}intersectSphere(e,i){_a.subVectors(e.center,this.origin);const s=_a.dot(this.direction),l=_a.dot(_a)-s*s,c=e.radius*e.radius;if(l>c)return null;const d=Math.sqrt(c-l),p=s-d,m=s+d;return m<0?null:p<0?this.at(m,i):this.at(p,i)}intersectsSphere(e){return e.radius<0?!1:this.distanceSqToPoint(e.center)<=e.radius*e.radius}distanceToPlane(e){const i=e.normal.dot(this.direction);if(i===0)return e.distanceToPoint(this.origin)===0?0:null;const s=-(this.origin.dot(e.normal)+e.constant)/i;return s>=0?s:null}intersectPlane(e,i){const s=this.distanceToPlane(e);return s===null?null:this.at(s,i)}intersectsPlane(e){const i=e.distanceToPoint(this.origin);return i===0||e.normal.dot(this.direction)*i<0}intersectBox(e,i){let s,l,c,d,p,m;const h=1/this.direction.x,v=1/this.direction.y,y=1/this.direction.z,g=this.origin;return h>=0?(s=(e.min.x-g.x)*h,l=(e.max.x-g.x)*h):(s=(e.max.x-g.x)*h,l=(e.min.x-g.x)*h),v>=0?(c=(e.min.y-g.y)*v,d=(e.max.y-g.y)*v):(c=(e.max.y-g.y)*v,d=(e.min.y-g.y)*v),s>d||c>l||((c>s||isNaN(s))&&(s=c),(d<l||isNaN(l))&&(l=d),y>=0?(p=(e.min.z-g.z)*y,m=(e.max.z-g.z)*y):(p=(e.max.z-g.z)*y,m=(e.min.z-g.z)*y),s>m||p>l)||((p>s||s!==s)&&(s=p),(m<l||l!==l)&&(l=m),l<0)?null:this.at(s>=0?s:l,i)}intersectsBox(e){return this.intersectBox(e,_a)!==null}intersectTriangle(e,i,s,l,c){Pd.subVectors(i,e),Mc.subVectors(s,e),Id.crossVectors(Pd,Mc);let d=this.direction.dot(Id),p;if(d>0){if(l)return null;p=1}else if(d<0)p=-1,d=-d;else return null;es.subVectors(this.origin,e);const m=p*this.direction.dot(Mc.crossVectors(es,Mc));if(m<0)return null;const h=p*this.direction.dot(Pd.cross(es));if(h<0||m+h>d)return null;const v=-p*es.dot(Id);return v<0?null:this.at(v/d,c)}applyMatrix4(e){return this.origin.applyMatrix4(e),this.direction.transformDirection(e),this}equals(e){return e.origin.equals(this.origin)&&e.direction.equals(this.direction)}clone(){return new this.constructor().copy(this)}}class yv extends Xr{constructor(e){super(),this.isMeshBasicMaterial=!0,this.type="MeshBasicMaterial",this.color=new At(16777215),this.map=null,this.lightMap=null,this.lightMapIntensity=1,this.aoMap=null,this.aoMapIntensity=1,this.specularMap=null,this.alphaMap=null,this.envMap=null,this.envMapRotation=new ji,this.combine=Q_,this.reflectivity=1,this.refractionRatio=.98,this.wireframe=!1,this.wireframeLinewidth=1,this.wireframeLinecap="round",this.wireframeLinejoin="round",this.fog=!0,this.setValues(e)}copy(e){return super.copy(e),this.color.copy(e.color),this.map=e.map,this.lightMap=e.lightMap,this.lightMapIntensity=e.lightMapIntensity,this.aoMap=e.aoMap,this.aoMapIntensity=e.aoMapIntensity,this.specularMap=e.specularMap,this.alphaMap=e.alphaMap,this.envMap=e.envMap,this.envMapRotation.copy(e.envMapRotation),this.combine=e.combine,this.reflectivity=e.reflectivity,this.refractionRatio=e.refractionRatio,this.wireframe=e.wireframe,this.wireframeLinewidth=e.wireframeLinewidth,this.wireframeLinecap=e.wireframeLinecap,this.wireframeLinejoin=e.wireframeLinejoin,this.fog=e.fog,this}}const f_=new Jt,Rs=new ep,Ec=new $c,d_=new K,Tc=new K,Ac=new K,Rc=new K,Fd=new K,wc=new K,h_=new K,Cc=new K;class Wi extends zn{constructor(e=new vi,i=new yv){super(),this.isMesh=!0,this.type="Mesh",this.geometry=e,this.material=i,this.morphTargetDictionary=void 0,this.morphTargetInfluences=void 0,this.count=1,this.updateMorphTargets()}copy(e,i){return super.copy(e,i),e.morphTargetInfluences!==void 0&&(this.morphTargetInfluences=e.morphTargetInfluences.slice()),e.morphTargetDictionary!==void 0&&(this.morphTargetDictionary=Object.assign({},e.morphTargetDictionary)),this.material=Array.isArray(e.material)?e.material.slice():e.material,this.geometry=e.geometry,this}updateMorphTargets(){const i=this.geometry.morphAttributes,s=Object.keys(i);if(s.length>0){const l=i[s[0]];if(l!==void 0){this.morphTargetInfluences=[],this.morphTargetDictionary={};for(let c=0,d=l.length;c<d;c++){const p=l[c].name||String(c);this.morphTargetInfluences.push(0),this.morphTargetDictionary[p]=c}}}}getVertexPosition(e,i){const s=this.geometry,l=s.attributes.position,c=s.morphAttributes.position,d=s.morphTargetsRelative;i.fromBufferAttribute(l,e);const p=this.morphTargetInfluences;if(c&&p){wc.set(0,0,0);for(let m=0,h=c.length;m<h;m++){const v=p[m],y=c[m];v!==0&&(Fd.fromBufferAttribute(y,e),d?wc.addScaledVector(Fd,v):wc.addScaledVector(Fd.sub(i),v))}i.add(wc)}return i}raycast(e,i){const s=this.geometry,l=this.material,c=this.matrixWorld;l!==void 0&&(s.boundingSphere===null&&s.computeBoundingSphere(),Ec.copy(s.boundingSphere),Ec.applyMatrix4(c),Rs.copy(e.ray).recast(e.near),!(Ec.containsPoint(Rs.origin)===!1&&(Rs.intersectSphere(Ec,d_)===null||Rs.origin.distanceToSquared(d_)>(e.far-e.near)**2))&&(f_.copy(c).invert(),Rs.copy(e.ray).applyMatrix4(f_),!(s.boundingBox!==null&&Rs.intersectsBox(s.boundingBox)===!1)&&this._computeIntersections(e,i,Rs)))}_computeIntersections(e,i,s){let l;const c=this.geometry,d=this.material,p=c.index,m=c.attributes.position,h=c.attributes.uv,v=c.attributes.uv1,y=c.attributes.normal,g=c.groups,x=c.drawRange;if(p!==null)if(Array.isArray(d))for(let E=0,w=g.length;E<w;E++){const b=g[E],S=d[b.materialIndex],C=Math.max(b.start,x.start),U=Math.min(p.count,Math.min(b.start+b.count,x.start+x.count));for(let N=C,V=U;N<V;N+=3){const H=p.getX(N),F=p.getX(N+1),T=p.getX(N+2);l=Dc(this,S,e,s,h,v,y,H,F,T),l&&(l.faceIndex=Math.floor(N/3),l.face.materialIndex=b.materialIndex,i.push(l))}}else{const E=Math.max(0,x.start),w=Math.min(p.count,x.start+x.count);for(let b=E,S=w;b<S;b+=3){const C=p.getX(b),U=p.getX(b+1),N=p.getX(b+2);l=Dc(this,d,e,s,h,v,y,C,U,N),l&&(l.faceIndex=Math.floor(b/3),i.push(l))}}else if(m!==void 0)if(Array.isArray(d))for(let E=0,w=g.length;E<w;E++){const b=g[E],S=d[b.materialIndex],C=Math.max(b.start,x.start),U=Math.min(m.count,Math.min(b.start+b.count,x.start+x.count));for(let N=C,V=U;N<V;N+=3){const H=N,F=N+1,T=N+2;l=Dc(this,S,e,s,h,v,y,H,F,T),l&&(l.faceIndex=Math.floor(N/3),l.face.materialIndex=b.materialIndex,i.push(l))}}else{const E=Math.max(0,x.start),w=Math.min(m.count,x.start+x.count);for(let b=E,S=w;b<S;b+=3){const C=b,U=b+1,N=b+2;l=Dc(this,d,e,s,h,v,y,C,U,N),l&&(l.faceIndex=Math.floor(b/3),i.push(l))}}}}function eb(o,e,i,s,l,c,d,p){let m;if(e.side===qn?m=s.intersectTriangle(d,c,l,!0,p):m=s.intersectTriangle(l,c,d,e.side===ss,p),m===null)return null;Cc.copy(p),Cc.applyMatrix4(o.matrixWorld);const h=i.ray.origin.distanceTo(Cc);return h<i.near||h>i.far?null:{distance:h,point:Cc.clone(),object:o}}function Dc(o,e,i,s,l,c,d,p,m,h){o.getVertexPosition(p,Tc),o.getVertexPosition(m,Ac),o.getVertexPosition(h,Rc);const v=eb(o,e,i,s,Tc,Ac,Rc,h_);if(v){const y=new K;Ci.getBarycoord(h_,Tc,Ac,Rc,y),l&&(v.uv=Ci.getInterpolatedAttribute(l,p,m,h,y,new ct)),c&&(v.uv1=Ci.getInterpolatedAttribute(c,p,m,h,y,new ct)),d&&(v.normal=Ci.getInterpolatedAttribute(d,p,m,h,y,new K),v.normal.dot(s.direction)>0&&v.normal.multiplyScalar(-1));const g={a:p,b:m,c:h,normal:new K,materialIndex:0};Ci.getNormal(Tc,Ac,Rc,g.normal),v.face=g,v.barycoord=y}return v}class tb extends Fn{constructor(e=null,i=1,s=1,l,c,d,p,m,h=An,v=An,y,g){super(null,d,p,m,h,v,l,c,y,g),this.isDataTexture=!0,this.image={data:e,width:i,height:s},this.generateMipmaps=!1,this.flipY=!1,this.unpackAlignment=1}}const zd=new K,nb=new K,ib=new ht;class ns{constructor(e=new K(1,0,0),i=0){this.isPlane=!0,this.normal=e,this.constant=i}set(e,i){return this.normal.copy(e),this.constant=i,this}setComponents(e,i,s,l){return this.normal.set(e,i,s),this.constant=l,this}setFromNormalAndCoplanarPoint(e,i){return this.normal.copy(e),this.constant=-i.dot(this.normal),this}setFromCoplanarPoints(e,i,s){const l=zd.subVectors(s,i).cross(nb.subVectors(e,i)).normalize();return this.setFromNormalAndCoplanarPoint(l,e),this}copy(e){return this.normal.copy(e.normal),this.constant=e.constant,this}normalize(){const e=1/this.normal.length();return this.normal.multiplyScalar(e),this.constant*=e,this}negate(){return this.constant*=-1,this.normal.negate(),this}distanceToPoint(e){return this.normal.dot(e)+this.constant}distanceToSphere(e){return this.distanceToPoint(e.center)-e.radius}projectPoint(e,i){return i.copy(e).addScaledVector(this.normal,-this.distanceToPoint(e))}intersectLine(e,i){const s=e.delta(zd),l=this.normal.dot(s);if(l===0)return this.distanceToPoint(e.start)===0?i.copy(e.start):null;const c=-(e.start.dot(this.normal)+this.constant)/l;return c<0||c>1?null:i.copy(e.start).addScaledVector(s,c)}intersectsLine(e){const i=this.distanceToPoint(e.start),s=this.distanceToPoint(e.end);return i<0&&s>0||s<0&&i>0}intersectsBox(e){return e.intersectsPlane(this)}intersectsSphere(e){return e.intersectsPlane(this)}coplanarPoint(e){return e.copy(this.normal).multiplyScalar(-this.constant)}applyMatrix4(e,i){const s=i||ib.getNormalMatrix(e),l=this.coplanarPoint(zd).applyMatrix4(e),c=this.normal.applyMatrix3(s).normalize();return this.constant=-l.dot(c),this}translate(e){return this.constant-=e.dot(this.normal),this}equals(e){return e.normal.equals(this.normal)&&e.constant===this.constant}clone(){return new this.constructor().copy(this)}}const ws=new $c,ab=new ct(.5,.5),Nc=new K;class tp{constructor(e=new ns,i=new ns,s=new ns,l=new ns,c=new ns,d=new ns){this.planes=[e,i,s,l,c,d]}set(e,i,s,l,c,d){const p=this.planes;return p[0].copy(e),p[1].copy(i),p[2].copy(s),p[3].copy(l),p[4].copy(c),p[5].copy(d),this}copy(e){const i=this.planes;for(let s=0;s<6;s++)i[s].copy(e.planes[s]);return this}setFromProjectionMatrix(e,i=Gi,s=!1){const l=this.planes,c=e.elements,d=c[0],p=c[1],m=c[2],h=c[3],v=c[4],y=c[5],g=c[6],x=c[7],E=c[8],w=c[9],b=c[10],S=c[11],C=c[12],U=c[13],N=c[14],V=c[15];if(l[0].setComponents(h-d,x-v,S-E,V-C).normalize(),l[1].setComponents(h+d,x+v,S+E,V+C).normalize(),l[2].setComponents(h+p,x+y,S+w,V+U).normalize(),l[3].setComponents(h-p,x-y,S-w,V-U).normalize(),s)l[4].setComponents(m,g,b,N).normalize(),l[5].setComponents(h-m,x-g,S-b,V-N).normalize();else if(l[4].setComponents(h-m,x-g,S-b,V-N).normalize(),i===Gi)l[5].setComponents(h+m,x+g,S+b,V+N).normalize();else if(i===Zo)l[5].setComponents(m,g,b,N).normalize();else throw new Error("THREE.Frustum.setFromProjectionMatrix(): Invalid coordinate system: "+i);return this}intersectsObject(e){if(e.boundingSphere!==void 0)e.boundingSphere===null&&e.computeBoundingSphere(),ws.copy(e.boundingSphere).applyMatrix4(e.matrixWorld);else{const i=e.geometry;i.boundingSphere===null&&i.computeBoundingSphere(),ws.copy(i.boundingSphere).applyMatrix4(e.matrixWorld)}return this.intersectsSphere(ws)}intersectsSprite(e){ws.center.set(0,0,0);const i=ab.distanceTo(e.center);return ws.radius=.7071067811865476+i,ws.applyMatrix4(e.matrixWorld),this.intersectsSphere(ws)}intersectsSphere(e){const i=this.planes,s=e.center,l=-e.radius;for(let c=0;c<6;c++)if(i[c].distanceToPoint(s)<l)return!1;return!0}intersectsBox(e){const i=this.planes;for(let s=0;s<6;s++){const l=i[s];if(Nc.x=l.normal.x>0?e.max.x:e.min.x,Nc.y=l.normal.y>0?e.max.y:e.min.y,Nc.z=l.normal.z>0?e.max.z:e.min.z,l.distanceToPoint(Nc)<0)return!1}return!0}containsPoint(e){const i=this.planes;for(let s=0;s<6;s++)if(i[s].distanceToPoint(e)<0)return!1;return!0}clone(){return new this.constructor().copy(this)}}class Sv extends Xr{constructor(e){super(),this.isPointsMaterial=!0,this.type="PointsMaterial",this.color=new At(16777215),this.map=null,this.alphaMap=null,this.size=1,this.sizeAttenuation=!0,this.fog=!0,this.setValues(e)}copy(e){return super.copy(e),this.color.copy(e.color),this.map=e.map,this.alphaMap=e.alphaMap,this.size=e.size,this.sizeAttenuation=e.sizeAttenuation,this.fog=e.fog,this}}const p_=new Jt,zh=new ep,Uc=new $c,Lc=new K;class sb extends zn{constructor(e=new vi,i=new Sv){super(),this.isPoints=!0,this.type="Points",this.geometry=e,this.material=i,this.morphTargetDictionary=void 0,this.morphTargetInfluences=void 0,this.updateMorphTargets()}copy(e,i){return super.copy(e,i),this.material=Array.isArray(e.material)?e.material.slice():e.material,this.geometry=e.geometry,this}raycast(e,i){const s=this.geometry,l=this.matrixWorld,c=e.params.Points.threshold,d=s.drawRange;if(s.boundingSphere===null&&s.computeBoundingSphere(),Uc.copy(s.boundingSphere),Uc.applyMatrix4(l),Uc.radius+=c,e.ray.intersectsSphere(Uc)===!1)return;p_.copy(l).invert(),zh.copy(e.ray).applyMatrix4(p_);const p=c/((this.scale.x+this.scale.y+this.scale.z)/3),m=p*p,h=s.index,y=s.attributes.position;if(h!==null){const g=Math.max(0,d.start),x=Math.min(h.count,d.start+d.count);for(let E=g,w=x;E<w;E++){const b=h.getX(E);Lc.fromBufferAttribute(y,b),m_(Lc,b,m,l,e,i,this)}}else{const g=Math.max(0,d.start),x=Math.min(y.count,d.start+d.count);for(let E=g,w=x;E<w;E++)Lc.fromBufferAttribute(y,E),m_(Lc,E,m,l,e,i,this)}}updateMorphTargets(){const i=this.geometry.morphAttributes,s=Object.keys(i);if(s.length>0){const l=i[s[0]];if(l!==void 0){this.morphTargetInfluences=[],this.morphTargetDictionary={};for(let c=0,d=l.length;c<d;c++){const p=l[c].name||String(c);this.morphTargetInfluences.push(0),this.morphTargetDictionary[p]=c}}}}}function m_(o,e,i,s,l,c,d){const p=zh.distanceSqToPoint(o);if(p<i){const m=new K;zh.closestPointToPoint(o,m),m.applyMatrix4(s);const h=l.ray.origin.distanceTo(m);if(h<l.near||h>l.far)return;c.push({distance:h,distanceToRay:Math.sqrt(p),point:m,index:e,face:null,faceIndex:null,barycoord:null,object:d})}}class bv extends Fn{constructor(e=[],i=Is,s,l,c,d,p,m,h,v){super(e,i,s,l,c,d,p,m,h,v),this.isCubeTexture=!0,this.flipY=!1}get images(){return this.image}set images(e){this.image=e}}class Ko extends Fn{constructor(e,i,s=Xi,l,c,d,p=An,m=An,h,v=Ma,y=1){if(v!==Ma&&v!==Ps)throw new Error("DepthTexture format must be either THREE.DepthFormat or THREE.DepthStencilFormat");const g={width:e,height:i,depth:y};super(g,l,c,d,p,m,v,s,h),this.isDepthTexture=!0,this.flipY=!1,this.generateMipmaps=!1,this.compareFunction=null}copy(e){return super.copy(e),this.source=new Jh(Object.assign({},e.image)),this.compareFunction=e.compareFunction,this}toJSON(e){const i=super.toJSON(e);return this.compareFunction!==null&&(i.compareFunction=this.compareFunction),i}}class rb extends Ko{constructor(e,i=Xi,s=Is,l,c,d=An,p=An,m,h=Ma){const v={width:e,height:e,depth:1},y=[v,v,v,v,v,v];super(e,e,i,s,l,c,d,p,m,h),this.image=y,this.isCubeDepthTexture=!0,this.isCubeTexture=!0}get images(){return this.image}set images(e){this.image=e}}class Mv extends Fn{constructor(e=null){super(),this.sourceTexture=e,this.isExternalTexture=!0}copy(e){return super.copy(e),this.sourceTexture=e.sourceTexture,this}}class $o extends vi{constructor(e=1,i=1,s=1,l=1,c=1,d=1){super(),this.type="BoxGeometry",this.parameters={width:e,height:i,depth:s,widthSegments:l,heightSegments:c,depthSegments:d};const p=this;l=Math.floor(l),c=Math.floor(c),d=Math.floor(d);const m=[],h=[],v=[],y=[];let g=0,x=0;E("z","y","x",-1,-1,s,i,e,d,c,0),E("z","y","x",1,-1,s,i,-e,d,c,1),E("x","z","y",1,1,e,s,i,l,d,2),E("x","z","y",1,-1,e,s,-i,l,d,3),E("x","y","z",1,-1,e,i,s,l,c,4),E("x","y","z",-1,-1,e,i,-s,l,c,5),this.setIndex(m),this.setAttribute("position",new _i(h,3)),this.setAttribute("normal",new _i(v,3)),this.setAttribute("uv",new _i(y,2));function E(w,b,S,C,U,N,V,H,F,T,D){const le=N/F,G=V/T,te=N/2,se=V/2,ue=H/2,ee=F+1,P=T+1;let z=0,ce=0;const pe=new K;for(let Ee=0;Ee<P;Ee++){const I=Ee*G-se;for(let Y=0;Y<ee;Y++){const ve=Y*le-te;pe[w]=ve*C,pe[b]=I*U,pe[S]=ue,h.push(pe.x,pe.y,pe.z),pe[w]=0,pe[b]=0,pe[S]=H>0?1:-1,v.push(pe.x,pe.y,pe.z),y.push(Y/F),y.push(1-Ee/T),z+=1}}for(let Ee=0;Ee<T;Ee++)for(let I=0;I<F;I++){const Y=g+I+ee*Ee,ve=g+I+ee*(Ee+1),Re=g+(I+1)+ee*(Ee+1),Fe=g+(I+1)+ee*Ee;m.push(Y,ve,Fe),m.push(ve,Re,Fe),ce+=6}p.addGroup(x,ce,D),x+=ce,g+=z}}copy(e){return super.copy(e),this.parameters=Object.assign({},e.parameters),this}static fromJSON(e){return new $o(e.width,e.height,e.depth,e.widthSegments,e.heightSegments,e.depthSegments)}}class eu extends vi{constructor(e=1,i=1,s=1,l=1){super(),this.type="PlaneGeometry",this.parameters={width:e,height:i,widthSegments:s,heightSegments:l};const c=e/2,d=i/2,p=Math.floor(s),m=Math.floor(l),h=p+1,v=m+1,y=e/p,g=i/m,x=[],E=[],w=[],b=[];for(let S=0;S<v;S++){const C=S*g-d;for(let U=0;U<h;U++){const N=U*y-c;E.push(N,-C,0),w.push(0,0,1),b.push(U/p),b.push(1-S/m)}}for(let S=0;S<m;S++)for(let C=0;C<p;C++){const U=C+h*S,N=C+h*(S+1),V=C+1+h*(S+1),H=C+1+h*S;x.push(U,N,H),x.push(N,V,H)}this.setIndex(x),this.setAttribute("position",new _i(E,3)),this.setAttribute("normal",new _i(w,3)),this.setAttribute("uv",new _i(b,2))}copy(e){return super.copy(e),this.parameters=Object.assign({},e.parameters),this}static fromJSON(e){return new eu(e.width,e.height,e.widthSegments,e.heightSegments)}}class np extends vi{constructor(e=1,i=.4,s=64,l=8,c=2,d=3){super(),this.type="TorusKnotGeometry",this.parameters={radius:e,tube:i,tubularSegments:s,radialSegments:l,p:c,q:d},s=Math.floor(s),l=Math.floor(l);const p=[],m=[],h=[],v=[],y=new K,g=new K,x=new K,E=new K,w=new K,b=new K,S=new K;for(let U=0;U<=s;++U){const N=U/s*c*Math.PI*2;C(N,c,d,e,x),C(N+.01,c,d,e,E),b.subVectors(E,x),S.addVectors(E,x),w.crossVectors(b,S),S.crossVectors(w,b),w.normalize(),S.normalize();for(let V=0;V<=l;++V){const H=V/l*Math.PI*2,F=-i*Math.cos(H),T=i*Math.sin(H);y.x=x.x+(F*S.x+T*w.x),y.y=x.y+(F*S.y+T*w.y),y.z=x.z+(F*S.z+T*w.z),m.push(y.x,y.y,y.z),g.subVectors(y,x).normalize(),h.push(g.x,g.y,g.z),v.push(U/s),v.push(V/l)}}for(let U=1;U<=s;U++)for(let N=1;N<=l;N++){const V=(l+1)*(U-1)+(N-1),H=(l+1)*U+(N-1),F=(l+1)*U+N,T=(l+1)*(U-1)+N;p.push(V,H,T),p.push(H,F,T)}this.setIndex(p),this.setAttribute("position",new _i(m,3)),this.setAttribute("normal",new _i(h,3)),this.setAttribute("uv",new _i(v,2));function C(U,N,V,H,F){const T=Math.cos(U),D=Math.sin(U),le=V/N*U,G=Math.cos(le);F.x=H*(2+G)*.5*T,F.y=H*(2+G)*D*.5,F.z=H*Math.sin(le)*.5}}copy(e){return super.copy(e),this.parameters=Object.assign({},e.parameters),this}static fromJSON(e){return new np(e.radius,e.tube,e.tubularSegments,e.radialSegments,e.p,e.q)}}function kr(o){const e={};for(const i in o){e[i]={};for(const s in o[i]){const l=o[i][s];l&&(l.isColor||l.isMatrix3||l.isMatrix4||l.isVector2||l.isVector3||l.isVector4||l.isTexture||l.isQuaternion)?l.isRenderTargetTexture?(at("UniformsUtils: Textures of render targets cannot be cloned via cloneUniforms() or mergeUniforms()."),e[i][s]=null):e[i][s]=l.clone():Array.isArray(l)?e[i][s]=l.slice():e[i][s]=l}}return e}function In(o){const e={};for(let i=0;i<o.length;i++){const s=kr(o[i]);for(const l in s)e[l]=s[l]}return e}function ob(o){const e=[];for(let i=0;i<o.length;i++)e.push(o[i].clone());return e}function Ev(o){const e=o.getRenderTarget();return e===null?o.outputColorSpace:e.isXRRenderTarget===!0?e.texture.colorSpace:Tt.workingColorSpace}const lb={clone:kr,merge:In};var cb=`void main() {
	gl_Position = projectionMatrix * modelViewMatrix * vec4( position, 1.0 );
}`,ub=`void main() {
	gl_FragColor = vec4( 1.0, 0.0, 0.0, 1.0 );
}`;class qi extends Xr{constructor(e){super(),this.isShaderMaterial=!0,this.type="ShaderMaterial",this.defines={},this.uniforms={},this.uniformsGroups=[],this.vertexShader=cb,this.fragmentShader=ub,this.linewidth=1,this.wireframe=!1,this.wireframeLinewidth=1,this.fog=!1,this.lights=!1,this.clipping=!1,this.forceSinglePass=!0,this.extensions={clipCullDistance:!1,multiDraw:!1},this.defaultAttributeValues={color:[1,1,1],uv:[0,0],uv1:[0,0]},this.index0AttributeName=void 0,this.uniformsNeedUpdate=!1,this.glslVersion=null,e!==void 0&&this.setValues(e)}copy(e){return super.copy(e),this.fragmentShader=e.fragmentShader,this.vertexShader=e.vertexShader,this.uniforms=kr(e.uniforms),this.uniformsGroups=ob(e.uniformsGroups),this.defines=Object.assign({},e.defines),this.wireframe=e.wireframe,this.wireframeLinewidth=e.wireframeLinewidth,this.fog=e.fog,this.lights=e.lights,this.clipping=e.clipping,this.extensions=Object.assign({},e.extensions),this.glslVersion=e.glslVersion,this.defaultAttributeValues=Object.assign({},e.defaultAttributeValues),this.index0AttributeName=e.index0AttributeName,this.uniformsNeedUpdate=e.uniformsNeedUpdate,this}toJSON(e){const i=super.toJSON(e);i.glslVersion=this.glslVersion,i.uniforms={};for(const l in this.uniforms){const d=this.uniforms[l].value;d&&d.isTexture?i.uniforms[l]={type:"t",value:d.toJSON(e).uuid}:d&&d.isColor?i.uniforms[l]={type:"c",value:d.getHex()}:d&&d.isVector2?i.uniforms[l]={type:"v2",value:d.toArray()}:d&&d.isVector3?i.uniforms[l]={type:"v3",value:d.toArray()}:d&&d.isVector4?i.uniforms[l]={type:"v4",value:d.toArray()}:d&&d.isMatrix3?i.uniforms[l]={type:"m3",value:d.toArray()}:d&&d.isMatrix4?i.uniforms[l]={type:"m4",value:d.toArray()}:i.uniforms[l]={value:d}}Object.keys(this.defines).length>0&&(i.defines=this.defines),i.vertexShader=this.vertexShader,i.fragmentShader=this.fragmentShader,i.lights=this.lights,i.clipping=this.clipping;const s={};for(const l in this.extensions)this.extensions[l]===!0&&(s[l]=!0);return Object.keys(s).length>0&&(i.extensions=s),i}}class fb extends qi{constructor(e){super(e),this.isRawShaderMaterial=!0,this.type="RawShaderMaterial"}}class db extends Xr{constructor(e){super(),this.isMeshStandardMaterial=!0,this.type="MeshStandardMaterial",this.defines={STANDARD:""},this.color=new At(16777215),this.roughness=1,this.metalness=0,this.map=null,this.lightMap=null,this.lightMapIntensity=1,this.aoMap=null,this.aoMapIntensity=1,this.emissive=new At(0),this.emissiveIntensity=1,this.emissiveMap=null,this.bumpMap=null,this.bumpScale=1,this.normalMap=null,this.normalMapType=hv,this.normalScale=new ct(1,1),this.displacementMap=null,this.displacementScale=1,this.displacementBias=0,this.roughnessMap=null,this.metalnessMap=null,this.alphaMap=null,this.envMap=null,this.envMapRotation=new ji,this.envMapIntensity=1,this.wireframe=!1,this.wireframeLinewidth=1,this.wireframeLinecap="round",this.wireframeLinejoin="round",this.flatShading=!1,this.fog=!0,this.setValues(e)}copy(e){return super.copy(e),this.defines={STANDARD:""},this.color.copy(e.color),this.roughness=e.roughness,this.metalness=e.metalness,this.map=e.map,this.lightMap=e.lightMap,this.lightMapIntensity=e.lightMapIntensity,this.aoMap=e.aoMap,this.aoMapIntensity=e.aoMapIntensity,this.emissive.copy(e.emissive),this.emissiveMap=e.emissiveMap,this.emissiveIntensity=e.emissiveIntensity,this.bumpMap=e.bumpMap,this.bumpScale=e.bumpScale,this.normalMap=e.normalMap,this.normalMapType=e.normalMapType,this.normalScale.copy(e.normalScale),this.displacementMap=e.displacementMap,this.displacementScale=e.displacementScale,this.displacementBias=e.displacementBias,this.roughnessMap=e.roughnessMap,this.metalnessMap=e.metalnessMap,this.alphaMap=e.alphaMap,this.envMap=e.envMap,this.envMapRotation.copy(e.envMapRotation),this.envMapIntensity=e.envMapIntensity,this.wireframe=e.wireframe,this.wireframeLinewidth=e.wireframeLinewidth,this.wireframeLinecap=e.wireframeLinecap,this.wireframeLinejoin=e.wireframeLinejoin,this.flatShading=e.flatShading,this.fog=e.fog,this}}class hb extends Xr{constructor(e){super(),this.isMeshDepthMaterial=!0,this.type="MeshDepthMaterial",this.depthPacking=bS,this.map=null,this.alphaMap=null,this.displacementMap=null,this.displacementScale=1,this.displacementBias=0,this.wireframe=!1,this.wireframeLinewidth=1,this.setValues(e)}copy(e){return super.copy(e),this.depthPacking=e.depthPacking,this.map=e.map,this.alphaMap=e.alphaMap,this.displacementMap=e.displacementMap,this.displacementScale=e.displacementScale,this.displacementBias=e.displacementBias,this.wireframe=e.wireframe,this.wireframeLinewidth=e.wireframeLinewidth,this}}class pb extends Xr{constructor(e){super(),this.isMeshDistanceMaterial=!0,this.type="MeshDistanceMaterial",this.map=null,this.alphaMap=null,this.displacementMap=null,this.displacementScale=1,this.displacementBias=0,this.setValues(e)}copy(e){return super.copy(e),this.map=e.map,this.alphaMap=e.alphaMap,this.displacementMap=e.displacementMap,this.displacementScale=e.displacementScale,this.displacementBias=e.displacementBias,this}}class Tv extends zn{constructor(e,i=1){super(),this.isLight=!0,this.type="Light",this.color=new At(e),this.intensity=i}dispose(){this.dispatchEvent({type:"dispose"})}copy(e,i){return super.copy(e,i),this.color.copy(e.color),this.intensity=e.intensity,this}toJSON(e){const i=super.toJSON(e);return i.object.color=this.color.getHex(),i.object.intensity=this.intensity,i}}const Bd=new Jt,g_=new K,__=new K;class mb{constructor(e){this.camera=e,this.intensity=1,this.bias=0,this.biasNode=null,this.normalBias=0,this.radius=1,this.blurSamples=8,this.mapSize=new ct(512,512),this.mapType=ri,this.map=null,this.mapPass=null,this.matrix=new Jt,this.autoUpdate=!0,this.needsUpdate=!1,this._frustum=new tp,this._frameExtents=new ct(1,1),this._viewportCount=1,this._viewports=[new nn(0,0,1,1)]}getViewportCount(){return this._viewportCount}getFrustum(){return this._frustum}updateMatrices(e){const i=this.camera,s=this.matrix;g_.setFromMatrixPosition(e.matrixWorld),i.position.copy(g_),__.setFromMatrixPosition(e.target.matrixWorld),i.lookAt(__),i.updateMatrixWorld(),Bd.multiplyMatrices(i.projectionMatrix,i.matrixWorldInverse),this._frustum.setFromProjectionMatrix(Bd,i.coordinateSystem,i.reversedDepth),i.coordinateSystem===Zo||i.reversedDepth?s.set(.5,0,0,.5,0,.5,0,.5,0,0,1,0,0,0,0,1):s.set(.5,0,0,.5,0,.5,0,.5,0,0,.5,.5,0,0,0,1),s.multiply(Bd)}getViewport(e){return this._viewports[e]}getFrameExtents(){return this._frameExtents}dispose(){this.map&&this.map.dispose(),this.mapPass&&this.mapPass.dispose()}copy(e){return this.camera=e.camera.clone(),this.intensity=e.intensity,this.bias=e.bias,this.radius=e.radius,this.autoUpdate=e.autoUpdate,this.needsUpdate=e.needsUpdate,this.normalBias=e.normalBias,this.blurSamples=e.blurSamples,this.mapSize.copy(e.mapSize),this.biasNode=e.biasNode,this}clone(){return new this.constructor().copy(this)}toJSON(){const e={};return this.intensity!==1&&(e.intensity=this.intensity),this.bias!==0&&(e.bias=this.bias),this.normalBias!==0&&(e.normalBias=this.normalBias),this.radius!==1&&(e.radius=this.radius),(this.mapSize.x!==512||this.mapSize.y!==512)&&(e.mapSize=this.mapSize.toArray()),e.camera=this.camera.toJSON(!1).object,delete e.camera.matrix,e}}const Oc=new K,Pc=new rs,Fi=new K;class Av extends zn{constructor(){super(),this.isCamera=!0,this.type="Camera",this.matrixWorldInverse=new Jt,this.projectionMatrix=new Jt,this.projectionMatrixInverse=new Jt,this.coordinateSystem=Gi,this._reversedDepth=!1}get reversedDepth(){return this._reversedDepth}copy(e,i){return super.copy(e,i),this.matrixWorldInverse.copy(e.matrixWorldInverse),this.projectionMatrix.copy(e.projectionMatrix),this.projectionMatrixInverse.copy(e.projectionMatrixInverse),this.coordinateSystem=e.coordinateSystem,this}getWorldDirection(e){return super.getWorldDirection(e).negate()}updateMatrixWorld(e){super.updateMatrixWorld(e),this.matrixWorld.decompose(Oc,Pc,Fi),Fi.x===1&&Fi.y===1&&Fi.z===1?this.matrixWorldInverse.copy(this.matrixWorld).invert():this.matrixWorldInverse.compose(Oc,Pc,Fi.set(1,1,1)).invert()}updateWorldMatrix(e,i){super.updateWorldMatrix(e,i),this.matrixWorld.decompose(Oc,Pc,Fi),Fi.x===1&&Fi.y===1&&Fi.z===1?this.matrixWorldInverse.copy(this.matrixWorld).invert():this.matrixWorldInverse.compose(Oc,Pc,Fi.set(1,1,1)).invert()}clone(){return new this.constructor().copy(this)}}const ts=new K,v_=new ct,x_=new ct;class si extends Av{constructor(e=50,i=1,s=.1,l=2e3){super(),this.isPerspectiveCamera=!0,this.type="PerspectiveCamera",this.fov=e,this.zoom=1,this.near=s,this.far=l,this.focus=10,this.aspect=i,this.view=null,this.filmGauge=35,this.filmOffset=0,this.updateProjectionMatrix()}copy(e,i){return super.copy(e,i),this.fov=e.fov,this.zoom=e.zoom,this.near=e.near,this.far=e.far,this.focus=e.focus,this.aspect=e.aspect,this.view=e.view===null?null:Object.assign({},e.view),this.filmGauge=e.filmGauge,this.filmOffset=e.filmOffset,this}setFocalLength(e){const i=.5*this.getFilmHeight()/e;this.fov=Fh*2*Math.atan(i),this.updateProjectionMatrix()}getFocalLength(){const e=Math.tan(Wc*.5*this.fov);return .5*this.getFilmHeight()/e}getEffectiveFOV(){return Fh*2*Math.atan(Math.tan(Wc*.5*this.fov)/this.zoom)}getFilmWidth(){return this.filmGauge*Math.min(this.aspect,1)}getFilmHeight(){return this.filmGauge/Math.max(this.aspect,1)}getViewBounds(e,i,s){ts.set(-1,-1,.5).applyMatrix4(this.projectionMatrixInverse),i.set(ts.x,ts.y).multiplyScalar(-e/ts.z),ts.set(1,1,.5).applyMatrix4(this.projectionMatrixInverse),s.set(ts.x,ts.y).multiplyScalar(-e/ts.z)}getViewSize(e,i){return this.getViewBounds(e,v_,x_),i.subVectors(x_,v_)}setViewOffset(e,i,s,l,c,d){this.aspect=e/i,this.view===null&&(this.view={enabled:!0,fullWidth:1,fullHeight:1,offsetX:0,offsetY:0,width:1,height:1}),this.view.enabled=!0,this.view.fullWidth=e,this.view.fullHeight=i,this.view.offsetX=s,this.view.offsetY=l,this.view.width=c,this.view.height=d,this.updateProjectionMatrix()}clearViewOffset(){this.view!==null&&(this.view.enabled=!1),this.updateProjectionMatrix()}updateProjectionMatrix(){const e=this.near;let i=e*Math.tan(Wc*.5*this.fov)/this.zoom,s=2*i,l=this.aspect*s,c=-.5*l;const d=this.view;if(this.view!==null&&this.view.enabled){const m=d.fullWidth,h=d.fullHeight;c+=d.offsetX*l/m,i-=d.offsetY*s/h,l*=d.width/m,s*=d.height/h}const p=this.filmOffset;p!==0&&(c+=e*p/this.getFilmWidth()),this.projectionMatrix.makePerspective(c,c+l,i,i-s,e,this.far,this.coordinateSystem,this.reversedDepth),this.projectionMatrixInverse.copy(this.projectionMatrix).invert()}toJSON(e){const i=super.toJSON(e);return i.object.fov=this.fov,i.object.zoom=this.zoom,i.object.near=this.near,i.object.far=this.far,i.object.focus=this.focus,i.object.aspect=this.aspect,this.view!==null&&(i.object.view=Object.assign({},this.view)),i.object.filmGauge=this.filmGauge,i.object.filmOffset=this.filmOffset,i}}class gb extends mb{constructor(){super(new si(90,1,.5,500)),this.isPointLightShadow=!0}}class _b extends Tv{constructor(e,i,s=0,l=2){super(e,i),this.isPointLight=!0,this.type="PointLight",this.distance=s,this.decay=l,this.shadow=new gb}get power(){return this.intensity*4*Math.PI}set power(e){this.intensity=e/(4*Math.PI)}dispose(){super.dispose(),this.shadow.dispose()}copy(e,i){return super.copy(e,i),this.distance=e.distance,this.decay=e.decay,this.shadow=e.shadow.clone(),this}toJSON(e){const i=super.toJSON(e);return i.object.distance=this.distance,i.object.decay=this.decay,i.object.shadow=this.shadow.toJSON(),i}}class Rv extends Av{constructor(e=-1,i=1,s=1,l=-1,c=.1,d=2e3){super(),this.isOrthographicCamera=!0,this.type="OrthographicCamera",this.zoom=1,this.view=null,this.left=e,this.right=i,this.top=s,this.bottom=l,this.near=c,this.far=d,this.updateProjectionMatrix()}copy(e,i){return super.copy(e,i),this.left=e.left,this.right=e.right,this.top=e.top,this.bottom=e.bottom,this.near=e.near,this.far=e.far,this.zoom=e.zoom,this.view=e.view===null?null:Object.assign({},e.view),this}setViewOffset(e,i,s,l,c,d){this.view===null&&(this.view={enabled:!0,fullWidth:1,fullHeight:1,offsetX:0,offsetY:0,width:1,height:1}),this.view.enabled=!0,this.view.fullWidth=e,this.view.fullHeight=i,this.view.offsetX=s,this.view.offsetY=l,this.view.width=c,this.view.height=d,this.updateProjectionMatrix()}clearViewOffset(){this.view!==null&&(this.view.enabled=!1),this.updateProjectionMatrix()}updateProjectionMatrix(){const e=(this.right-this.left)/(2*this.zoom),i=(this.top-this.bottom)/(2*this.zoom),s=(this.right+this.left)/2,l=(this.top+this.bottom)/2;let c=s-e,d=s+e,p=l+i,m=l-i;if(this.view!==null&&this.view.enabled){const h=(this.right-this.left)/this.view.fullWidth/this.zoom,v=(this.top-this.bottom)/this.view.fullHeight/this.zoom;c+=h*this.view.offsetX,d=c+h*this.view.width,p-=v*this.view.offsetY,m=p-v*this.view.height}this.projectionMatrix.makeOrthographic(c,d,p,m,this.near,this.far,this.coordinateSystem,this.reversedDepth),this.projectionMatrixInverse.copy(this.projectionMatrix).invert()}toJSON(e){const i=super.toJSON(e);return i.object.zoom=this.zoom,i.object.left=this.left,i.object.right=this.right,i.object.top=this.top,i.object.bottom=this.bottom,i.object.near=this.near,i.object.far=this.far,this.view!==null&&(i.object.view=Object.assign({},this.view)),i}}class vb extends Tv{constructor(e,i){super(e,i),this.isAmbientLight=!0,this.type="AmbientLight"}}const Nr=-90,Ur=1;class xb extends zn{constructor(e,i,s){super(),this.type="CubeCamera",this.renderTarget=s,this.coordinateSystem=null,this.activeMipmapLevel=0;const l=new si(Nr,Ur,e,i);l.layers=this.layers,this.add(l);const c=new si(Nr,Ur,e,i);c.layers=this.layers,this.add(c);const d=new si(Nr,Ur,e,i);d.layers=this.layers,this.add(d);const p=new si(Nr,Ur,e,i);p.layers=this.layers,this.add(p);const m=new si(Nr,Ur,e,i);m.layers=this.layers,this.add(m);const h=new si(Nr,Ur,e,i);h.layers=this.layers,this.add(h)}updateCoordinateSystem(){const e=this.coordinateSystem,i=this.children.concat(),[s,l,c,d,p,m]=i;for(const h of i)this.remove(h);if(e===Gi)s.up.set(0,1,0),s.lookAt(1,0,0),l.up.set(0,1,0),l.lookAt(-1,0,0),c.up.set(0,0,-1),c.lookAt(0,1,0),d.up.set(0,0,1),d.lookAt(0,-1,0),p.up.set(0,1,0),p.lookAt(0,0,1),m.up.set(0,1,0),m.lookAt(0,0,-1);else if(e===Zo)s.up.set(0,-1,0),s.lookAt(-1,0,0),l.up.set(0,-1,0),l.lookAt(1,0,0),c.up.set(0,0,1),c.lookAt(0,1,0),d.up.set(0,0,-1),d.lookAt(0,-1,0),p.up.set(0,-1,0),p.lookAt(0,0,1),m.up.set(0,-1,0),m.lookAt(0,0,-1);else throw new Error("THREE.CubeCamera.updateCoordinateSystem(): Invalid coordinate system: "+e);for(const h of i)this.add(h),h.updateMatrixWorld()}update(e,i){this.parent===null&&this.updateMatrixWorld();const{renderTarget:s,activeMipmapLevel:l}=this;this.coordinateSystem!==e.coordinateSystem&&(this.coordinateSystem=e.coordinateSystem,this.updateCoordinateSystem());const[c,d,p,m,h,v]=this.children,y=e.getRenderTarget(),g=e.getActiveCubeFace(),x=e.getActiveMipmapLevel(),E=e.xr.enabled;e.xr.enabled=!1;const w=s.texture.generateMipmaps;s.texture.generateMipmaps=!1;let b=!1;e.isWebGLRenderer===!0?b=e.state.buffers.depth.getReversed():b=e.reversedDepthBuffer,e.setRenderTarget(s,0,l),b&&e.autoClear===!1&&e.clearDepth(),e.render(i,c),e.setRenderTarget(s,1,l),b&&e.autoClear===!1&&e.clearDepth(),e.render(i,d),e.setRenderTarget(s,2,l),b&&e.autoClear===!1&&e.clearDepth(),e.render(i,p),e.setRenderTarget(s,3,l),b&&e.autoClear===!1&&e.clearDepth(),e.render(i,m),e.setRenderTarget(s,4,l),b&&e.autoClear===!1&&e.clearDepth(),e.render(i,h),s.texture.generateMipmaps=w,e.setRenderTarget(s,5,l),b&&e.autoClear===!1&&e.clearDepth(),e.render(i,v),e.setRenderTarget(y,g,x),e.xr.enabled=E,s.texture.needsPMREMUpdate=!0}}class yb extends si{constructor(e=[]){super(),this.isArrayCamera=!0,this.isMultiViewCamera=!1,this.cameras=e}}class y_{constructor(e=1,i=0,s=0){this.radius=e,this.phi=i,this.theta=s}set(e,i,s){return this.radius=e,this.phi=i,this.theta=s,this}copy(e){return this.radius=e.radius,this.phi=e.phi,this.theta=e.theta,this}makeSafe(){return this.phi=vt(this.phi,1e-6,Math.PI-1e-6),this}setFromVector3(e){return this.setFromCartesianCoords(e.x,e.y,e.z)}setFromCartesianCoords(e,i,s){return this.radius=Math.sqrt(e*e+i*i+s*s),this.radius===0?(this.theta=0,this.phi=0):(this.theta=Math.atan2(e,s),this.phi=Math.acos(vt(i/this.radius,-1,1))),this}clone(){return new this.constructor().copy(this)}}class Sb extends Fs{constructor(e,i=null){super(),this.object=e,this.domElement=i,this.enabled=!0,this.state=-1,this.keys={},this.mouseButtons={LEFT:null,MIDDLE:null,RIGHT:null},this.touches={ONE:null,TWO:null}}connect(e){if(e===void 0){at("Controls: connect() now requires an element.");return}this.domElement!==null&&this.disconnect(),this.domElement=e}disconnect(){}dispose(){}update(){}}function S_(o,e,i,s){const l=bb(s);switch(i){case uv:return o*e;case dv:return o*e/l.components*l.byteLength;case qh:return o*e/l.components*l.byteLength;case Gr:return o*e*2/l.components*l.byteLength;case Yh:return o*e*2/l.components*l.byteLength;case fv:return o*e*3/l.components*l.byteLength;case Di:return o*e*4/l.components*l.byteLength;case Zh:return o*e*4/l.components*l.byteLength;case Vc:case kc:return Math.floor((o+3)/4)*Math.floor((e+3)/4)*8;case Xc:case jc:return Math.floor((o+3)/4)*Math.floor((e+3)/4)*16;case rh:case lh:return Math.max(o,16)*Math.max(e,8)/4;case sh:case oh:return Math.max(o,8)*Math.max(e,8)/2;case ch:case uh:case dh:case hh:return Math.floor((o+3)/4)*Math.floor((e+3)/4)*8;case fh:case ph:case mh:return Math.floor((o+3)/4)*Math.floor((e+3)/4)*16;case gh:return Math.floor((o+3)/4)*Math.floor((e+3)/4)*16;case _h:return Math.floor((o+4)/5)*Math.floor((e+3)/4)*16;case vh:return Math.floor((o+4)/5)*Math.floor((e+4)/5)*16;case xh:return Math.floor((o+5)/6)*Math.floor((e+4)/5)*16;case yh:return Math.floor((o+5)/6)*Math.floor((e+5)/6)*16;case Sh:return Math.floor((o+7)/8)*Math.floor((e+4)/5)*16;case bh:return Math.floor((o+7)/8)*Math.floor((e+5)/6)*16;case Mh:return Math.floor((o+7)/8)*Math.floor((e+7)/8)*16;case Eh:return Math.floor((o+9)/10)*Math.floor((e+4)/5)*16;case Th:return Math.floor((o+9)/10)*Math.floor((e+5)/6)*16;case Ah:return Math.floor((o+9)/10)*Math.floor((e+7)/8)*16;case Rh:return Math.floor((o+9)/10)*Math.floor((e+9)/10)*16;case wh:return Math.floor((o+11)/12)*Math.floor((e+9)/10)*16;case Ch:return Math.floor((o+11)/12)*Math.floor((e+11)/12)*16;case Dh:case Nh:case Uh:return Math.ceil(o/4)*Math.ceil(e/4)*16;case Lh:case Oh:return Math.ceil(o/4)*Math.ceil(e/4)*8;case Ph:case Ih:return Math.ceil(o/4)*Math.ceil(e/4)*16}throw new Error(`Unable to determine texture byte length for ${i} format.`)}function bb(o){switch(o){case ri:case rv:return{byteLength:1,components:1};case qo:case ov:case ba:return{byteLength:2,components:1};case jh:case Wh:return{byteLength:2,components:4};case Xi:case Xh:case Hi:return{byteLength:4,components:1};case lv:case cv:return{byteLength:4,components:3}}throw new Error(`Unknown texture type ${o}.`)}typeof __THREE_DEVTOOLS__<"u"&&__THREE_DEVTOOLS__.dispatchEvent(new CustomEvent("register",{detail:{revision:kh}}));typeof window<"u"&&(window.__THREE__?at("WARNING: Multiple instances of Three.js being imported."):window.__THREE__=kh);/**
 * @license
 * Copyright 2010-2026 Three.js Authors
 * SPDX-License-Identifier: MIT
 */function wv(){let o=null,e=!1,i=null,s=null;function l(c,d){i(c,d),s=o.requestAnimationFrame(l)}return{start:function(){e!==!0&&i!==null&&(s=o.requestAnimationFrame(l),e=!0)},stop:function(){o.cancelAnimationFrame(s),e=!1},setAnimationLoop:function(c){i=c},setContext:function(c){o=c}}}function Mb(o){const e=new WeakMap;function i(p,m){const h=p.array,v=p.usage,y=h.byteLength,g=o.createBuffer();o.bindBuffer(m,g),o.bufferData(m,h,v),p.onUploadCallback();let x;if(h instanceof Float32Array)x=o.FLOAT;else if(typeof Float16Array<"u"&&h instanceof Float16Array)x=o.HALF_FLOAT;else if(h instanceof Uint16Array)p.isFloat16BufferAttribute?x=o.HALF_FLOAT:x=o.UNSIGNED_SHORT;else if(h instanceof Int16Array)x=o.SHORT;else if(h instanceof Uint32Array)x=o.UNSIGNED_INT;else if(h instanceof Int32Array)x=o.INT;else if(h instanceof Int8Array)x=o.BYTE;else if(h instanceof Uint8Array)x=o.UNSIGNED_BYTE;else if(h instanceof Uint8ClampedArray)x=o.UNSIGNED_BYTE;else throw new Error("THREE.WebGLAttributes: Unsupported buffer data format: "+h);return{buffer:g,type:x,bytesPerElement:h.BYTES_PER_ELEMENT,version:p.version,size:y}}function s(p,m,h){const v=m.array,y=m.updateRanges;if(o.bindBuffer(h,p),y.length===0)o.bufferSubData(h,0,v);else{y.sort((x,E)=>x.start-E.start);let g=0;for(let x=1;x<y.length;x++){const E=y[g],w=y[x];w.start<=E.start+E.count+1?E.count=Math.max(E.count,w.start+w.count-E.start):(++g,y[g]=w)}y.length=g+1;for(let x=0,E=y.length;x<E;x++){const w=y[x];o.bufferSubData(h,w.start*v.BYTES_PER_ELEMENT,v,w.start,w.count)}m.clearUpdateRanges()}m.onUploadCallback()}function l(p){return p.isInterleavedBufferAttribute&&(p=p.data),e.get(p)}function c(p){p.isInterleavedBufferAttribute&&(p=p.data);const m=e.get(p);m&&(o.deleteBuffer(m.buffer),e.delete(p))}function d(p,m){if(p.isInterleavedBufferAttribute&&(p=p.data),p.isGLBufferAttribute){const v=e.get(p);(!v||v.version<p.version)&&e.set(p,{buffer:p.buffer,type:p.type,bytesPerElement:p.elementSize,version:p.version});return}const h=e.get(p);if(h===void 0)e.set(p,i(p,m));else if(h.version<p.version){if(h.size!==p.array.byteLength)throw new Error("THREE.WebGLAttributes: The size of the buffer attribute's array buffer does not match the original size. Resizing buffer attributes is not supported.");s(h.buffer,p,m),h.version=p.version}}return{get:l,remove:c,update:d}}var Eb=`#ifdef USE_ALPHAHASH
	if ( diffuseColor.a < getAlphaHashThreshold( vPosition ) ) discard;
#endif`,Tb=`#ifdef USE_ALPHAHASH
	const float ALPHA_HASH_SCALE = 0.05;
	float hash2D( vec2 value ) {
		return fract( 1.0e4 * sin( 17.0 * value.x + 0.1 * value.y ) * ( 0.1 + abs( sin( 13.0 * value.y + value.x ) ) ) );
	}
	float hash3D( vec3 value ) {
		return hash2D( vec2( hash2D( value.xy ), value.z ) );
	}
	float getAlphaHashThreshold( vec3 position ) {
		float maxDeriv = max(
			length( dFdx( position.xyz ) ),
			length( dFdy( position.xyz ) )
		);
		float pixScale = 1.0 / ( ALPHA_HASH_SCALE * maxDeriv );
		vec2 pixScales = vec2(
			exp2( floor( log2( pixScale ) ) ),
			exp2( ceil( log2( pixScale ) ) )
		);
		vec2 alpha = vec2(
			hash3D( floor( pixScales.x * position.xyz ) ),
			hash3D( floor( pixScales.y * position.xyz ) )
		);
		float lerpFactor = fract( log2( pixScale ) );
		float x = ( 1.0 - lerpFactor ) * alpha.x + lerpFactor * alpha.y;
		float a = min( lerpFactor, 1.0 - lerpFactor );
		vec3 cases = vec3(
			x * x / ( 2.0 * a * ( 1.0 - a ) ),
			( x - 0.5 * a ) / ( 1.0 - a ),
			1.0 - ( ( 1.0 - x ) * ( 1.0 - x ) / ( 2.0 * a * ( 1.0 - a ) ) )
		);
		float threshold = ( x < ( 1.0 - a ) )
			? ( ( x < a ) ? cases.x : cases.y )
			: cases.z;
		return clamp( threshold , 1.0e-6, 1.0 );
	}
#endif`,Ab=`#ifdef USE_ALPHAMAP
	diffuseColor.a *= texture2D( alphaMap, vAlphaMapUv ).g;
#endif`,Rb=`#ifdef USE_ALPHAMAP
	uniform sampler2D alphaMap;
#endif`,wb=`#ifdef USE_ALPHATEST
	#ifdef ALPHA_TO_COVERAGE
	diffuseColor.a = smoothstep( alphaTest, alphaTest + fwidth( diffuseColor.a ), diffuseColor.a );
	if ( diffuseColor.a == 0.0 ) discard;
	#else
	if ( diffuseColor.a < alphaTest ) discard;
	#endif
#endif`,Cb=`#ifdef USE_ALPHATEST
	uniform float alphaTest;
#endif`,Db=`#ifdef USE_AOMAP
	float ambientOcclusion = ( texture2D( aoMap, vAoMapUv ).r - 1.0 ) * aoMapIntensity + 1.0;
	reflectedLight.indirectDiffuse *= ambientOcclusion;
	#if defined( USE_CLEARCOAT ) 
		clearcoatSpecularIndirect *= ambientOcclusion;
	#endif
	#if defined( USE_SHEEN ) 
		sheenSpecularIndirect *= ambientOcclusion;
	#endif
	#if defined( USE_ENVMAP ) && defined( STANDARD )
		float dotNV = saturate( dot( geometryNormal, geometryViewDir ) );
		reflectedLight.indirectSpecular *= computeSpecularOcclusion( dotNV, ambientOcclusion, material.roughness );
	#endif
#endif`,Nb=`#ifdef USE_AOMAP
	uniform sampler2D aoMap;
	uniform float aoMapIntensity;
#endif`,Ub=`#ifdef USE_BATCHING
	#if ! defined( GL_ANGLE_multi_draw )
	#define gl_DrawID _gl_DrawID
	uniform int _gl_DrawID;
	#endif
	uniform highp sampler2D batchingTexture;
	uniform highp usampler2D batchingIdTexture;
	mat4 getBatchingMatrix( const in float i ) {
		int size = textureSize( batchingTexture, 0 ).x;
		int j = int( i ) * 4;
		int x = j % size;
		int y = j / size;
		vec4 v1 = texelFetch( batchingTexture, ivec2( x, y ), 0 );
		vec4 v2 = texelFetch( batchingTexture, ivec2( x + 1, y ), 0 );
		vec4 v3 = texelFetch( batchingTexture, ivec2( x + 2, y ), 0 );
		vec4 v4 = texelFetch( batchingTexture, ivec2( x + 3, y ), 0 );
		return mat4( v1, v2, v3, v4 );
	}
	float getIndirectIndex( const in int i ) {
		int size = textureSize( batchingIdTexture, 0 ).x;
		int x = i % size;
		int y = i / size;
		return float( texelFetch( batchingIdTexture, ivec2( x, y ), 0 ).r );
	}
#endif
#ifdef USE_BATCHING_COLOR
	uniform sampler2D batchingColorTexture;
	vec4 getBatchingColor( const in float i ) {
		int size = textureSize( batchingColorTexture, 0 ).x;
		int j = int( i );
		int x = j % size;
		int y = j / size;
		return texelFetch( batchingColorTexture, ivec2( x, y ), 0 );
	}
#endif`,Lb=`#ifdef USE_BATCHING
	mat4 batchingMatrix = getBatchingMatrix( getIndirectIndex( gl_DrawID ) );
#endif`,Ob=`vec3 transformed = vec3( position );
#ifdef USE_ALPHAHASH
	vPosition = vec3( position );
#endif`,Pb=`vec3 objectNormal = vec3( normal );
#ifdef USE_TANGENT
	vec3 objectTangent = vec3( tangent.xyz );
#endif`,Ib=`float G_BlinnPhong_Implicit( ) {
	return 0.25;
}
float D_BlinnPhong( const in float shininess, const in float dotNH ) {
	return RECIPROCAL_PI * ( shininess * 0.5 + 1.0 ) * pow( dotNH, shininess );
}
vec3 BRDF_BlinnPhong( const in vec3 lightDir, const in vec3 viewDir, const in vec3 normal, const in vec3 specularColor, const in float shininess ) {
	vec3 halfDir = normalize( lightDir + viewDir );
	float dotNH = saturate( dot( normal, halfDir ) );
	float dotVH = saturate( dot( viewDir, halfDir ) );
	vec3 F = F_Schlick( specularColor, 1.0, dotVH );
	float G = G_BlinnPhong_Implicit( );
	float D = D_BlinnPhong( shininess, dotNH );
	return F * ( G * D );
} // validated`,Fb=`#ifdef USE_IRIDESCENCE
	const mat3 XYZ_TO_REC709 = mat3(
		 3.2404542, -0.9692660,  0.0556434,
		-1.5371385,  1.8760108, -0.2040259,
		-0.4985314,  0.0415560,  1.0572252
	);
	vec3 Fresnel0ToIor( vec3 fresnel0 ) {
		vec3 sqrtF0 = sqrt( fresnel0 );
		return ( vec3( 1.0 ) + sqrtF0 ) / ( vec3( 1.0 ) - sqrtF0 );
	}
	vec3 IorToFresnel0( vec3 transmittedIor, float incidentIor ) {
		return pow2( ( transmittedIor - vec3( incidentIor ) ) / ( transmittedIor + vec3( incidentIor ) ) );
	}
	float IorToFresnel0( float transmittedIor, float incidentIor ) {
		return pow2( ( transmittedIor - incidentIor ) / ( transmittedIor + incidentIor ));
	}
	vec3 evalSensitivity( float OPD, vec3 shift ) {
		float phase = 2.0 * PI * OPD * 1.0e-9;
		vec3 val = vec3( 5.4856e-13, 4.4201e-13, 5.2481e-13 );
		vec3 pos = vec3( 1.6810e+06, 1.7953e+06, 2.2084e+06 );
		vec3 var = vec3( 4.3278e+09, 9.3046e+09, 6.6121e+09 );
		vec3 xyz = val * sqrt( 2.0 * PI * var ) * cos( pos * phase + shift ) * exp( - pow2( phase ) * var );
		xyz.x += 9.7470e-14 * sqrt( 2.0 * PI * 4.5282e+09 ) * cos( 2.2399e+06 * phase + shift[ 0 ] ) * exp( - 4.5282e+09 * pow2( phase ) );
		xyz /= 1.0685e-7;
		vec3 rgb = XYZ_TO_REC709 * xyz;
		return rgb;
	}
	vec3 evalIridescence( float outsideIOR, float eta2, float cosTheta1, float thinFilmThickness, vec3 baseF0 ) {
		vec3 I;
		float iridescenceIOR = mix( outsideIOR, eta2, smoothstep( 0.0, 0.03, thinFilmThickness ) );
		float sinTheta2Sq = pow2( outsideIOR / iridescenceIOR ) * ( 1.0 - pow2( cosTheta1 ) );
		float cosTheta2Sq = 1.0 - sinTheta2Sq;
		if ( cosTheta2Sq < 0.0 ) {
			return vec3( 1.0 );
		}
		float cosTheta2 = sqrt( cosTheta2Sq );
		float R0 = IorToFresnel0( iridescenceIOR, outsideIOR );
		float R12 = F_Schlick( R0, 1.0, cosTheta1 );
		float T121 = 1.0 - R12;
		float phi12 = 0.0;
		if ( iridescenceIOR < outsideIOR ) phi12 = PI;
		float phi21 = PI - phi12;
		vec3 baseIOR = Fresnel0ToIor( clamp( baseF0, 0.0, 0.9999 ) );		vec3 R1 = IorToFresnel0( baseIOR, iridescenceIOR );
		vec3 R23 = F_Schlick( R1, 1.0, cosTheta2 );
		vec3 phi23 = vec3( 0.0 );
		if ( baseIOR[ 0 ] < iridescenceIOR ) phi23[ 0 ] = PI;
		if ( baseIOR[ 1 ] < iridescenceIOR ) phi23[ 1 ] = PI;
		if ( baseIOR[ 2 ] < iridescenceIOR ) phi23[ 2 ] = PI;
		float OPD = 2.0 * iridescenceIOR * thinFilmThickness * cosTheta2;
		vec3 phi = vec3( phi21 ) + phi23;
		vec3 R123 = clamp( R12 * R23, 1e-5, 0.9999 );
		vec3 r123 = sqrt( R123 );
		vec3 Rs = pow2( T121 ) * R23 / ( vec3( 1.0 ) - R123 );
		vec3 C0 = R12 + Rs;
		I = C0;
		vec3 Cm = Rs - T121;
		for ( int m = 1; m <= 2; ++ m ) {
			Cm *= r123;
			vec3 Sm = 2.0 * evalSensitivity( float( m ) * OPD, float( m ) * phi );
			I += Cm * Sm;
		}
		return max( I, vec3( 0.0 ) );
	}
#endif`,zb=`#ifdef USE_BUMPMAP
	uniform sampler2D bumpMap;
	uniform float bumpScale;
	vec2 dHdxy_fwd() {
		vec2 dSTdx = dFdx( vBumpMapUv );
		vec2 dSTdy = dFdy( vBumpMapUv );
		float Hll = bumpScale * texture2D( bumpMap, vBumpMapUv ).x;
		float dBx = bumpScale * texture2D( bumpMap, vBumpMapUv + dSTdx ).x - Hll;
		float dBy = bumpScale * texture2D( bumpMap, vBumpMapUv + dSTdy ).x - Hll;
		return vec2( dBx, dBy );
	}
	vec3 perturbNormalArb( vec3 surf_pos, vec3 surf_norm, vec2 dHdxy, float faceDirection ) {
		vec3 vSigmaX = normalize( dFdx( surf_pos.xyz ) );
		vec3 vSigmaY = normalize( dFdy( surf_pos.xyz ) );
		vec3 vN = surf_norm;
		vec3 R1 = cross( vSigmaY, vN );
		vec3 R2 = cross( vN, vSigmaX );
		float fDet = dot( vSigmaX, R1 ) * faceDirection;
		vec3 vGrad = sign( fDet ) * ( dHdxy.x * R1 + dHdxy.y * R2 );
		return normalize( abs( fDet ) * surf_norm - vGrad );
	}
#endif`,Bb=`#if NUM_CLIPPING_PLANES > 0
	vec4 plane;
	#ifdef ALPHA_TO_COVERAGE
		float distanceToPlane, distanceGradient;
		float clipOpacity = 1.0;
		#pragma unroll_loop_start
		for ( int i = 0; i < UNION_CLIPPING_PLANES; i ++ ) {
			plane = clippingPlanes[ i ];
			distanceToPlane = - dot( vClipPosition, plane.xyz ) + plane.w;
			distanceGradient = fwidth( distanceToPlane ) / 2.0;
			clipOpacity *= smoothstep( - distanceGradient, distanceGradient, distanceToPlane );
			if ( clipOpacity == 0.0 ) discard;
		}
		#pragma unroll_loop_end
		#if UNION_CLIPPING_PLANES < NUM_CLIPPING_PLANES
			float unionClipOpacity = 1.0;
			#pragma unroll_loop_start
			for ( int i = UNION_CLIPPING_PLANES; i < NUM_CLIPPING_PLANES; i ++ ) {
				plane = clippingPlanes[ i ];
				distanceToPlane = - dot( vClipPosition, plane.xyz ) + plane.w;
				distanceGradient = fwidth( distanceToPlane ) / 2.0;
				unionClipOpacity *= 1.0 - smoothstep( - distanceGradient, distanceGradient, distanceToPlane );
			}
			#pragma unroll_loop_end
			clipOpacity *= 1.0 - unionClipOpacity;
		#endif
		diffuseColor.a *= clipOpacity;
		if ( diffuseColor.a == 0.0 ) discard;
	#else
		#pragma unroll_loop_start
		for ( int i = 0; i < UNION_CLIPPING_PLANES; i ++ ) {
			plane = clippingPlanes[ i ];
			if ( dot( vClipPosition, plane.xyz ) > plane.w ) discard;
		}
		#pragma unroll_loop_end
		#if UNION_CLIPPING_PLANES < NUM_CLIPPING_PLANES
			bool clipped = true;
			#pragma unroll_loop_start
			for ( int i = UNION_CLIPPING_PLANES; i < NUM_CLIPPING_PLANES; i ++ ) {
				plane = clippingPlanes[ i ];
				clipped = ( dot( vClipPosition, plane.xyz ) > plane.w ) && clipped;
			}
			#pragma unroll_loop_end
			if ( clipped ) discard;
		#endif
	#endif
#endif`,Hb=`#if NUM_CLIPPING_PLANES > 0
	varying vec3 vClipPosition;
	uniform vec4 clippingPlanes[ NUM_CLIPPING_PLANES ];
#endif`,Gb=`#if NUM_CLIPPING_PLANES > 0
	varying vec3 vClipPosition;
#endif`,Vb=`#if NUM_CLIPPING_PLANES > 0
	vClipPosition = - mvPosition.xyz;
#endif`,kb=`#if defined( USE_COLOR ) || defined( USE_COLOR_ALPHA )
	diffuseColor *= vColor;
#endif`,Xb=`#if defined( USE_COLOR ) || defined( USE_COLOR_ALPHA )
	varying vec4 vColor;
#endif`,jb=`#if defined( USE_COLOR ) || defined( USE_COLOR_ALPHA ) || defined( USE_INSTANCING_COLOR ) || defined( USE_BATCHING_COLOR )
	varying vec4 vColor;
#endif`,Wb=`#if defined( USE_COLOR ) || defined( USE_COLOR_ALPHA ) || defined( USE_INSTANCING_COLOR ) || defined( USE_BATCHING_COLOR )
	vColor = vec4( 1.0 );
#endif
#ifdef USE_COLOR_ALPHA
	vColor *= color;
#elif defined( USE_COLOR )
	vColor.rgb *= color;
#endif
#ifdef USE_INSTANCING_COLOR
	vColor.rgb *= instanceColor.rgb;
#endif
#ifdef USE_BATCHING_COLOR
	vColor *= getBatchingColor( getIndirectIndex( gl_DrawID ) );
#endif`,qb=`#define PI 3.141592653589793
#define PI2 6.283185307179586
#define PI_HALF 1.5707963267948966
#define RECIPROCAL_PI 0.3183098861837907
#define RECIPROCAL_PI2 0.15915494309189535
#define EPSILON 1e-6
#ifndef saturate
#define saturate( a ) clamp( a, 0.0, 1.0 )
#endif
#define whiteComplement( a ) ( 1.0 - saturate( a ) )
float pow2( const in float x ) { return x*x; }
vec3 pow2( const in vec3 x ) { return x*x; }
float pow3( const in float x ) { return x*x*x; }
float pow4( const in float x ) { float x2 = x*x; return x2*x2; }
float max3( const in vec3 v ) { return max( max( v.x, v.y ), v.z ); }
float average( const in vec3 v ) { return dot( v, vec3( 0.3333333 ) ); }
highp float rand( const in vec2 uv ) {
	const highp float a = 12.9898, b = 78.233, c = 43758.5453;
	highp float dt = dot( uv.xy, vec2( a,b ) ), sn = mod( dt, PI );
	return fract( sin( sn ) * c );
}
#ifdef HIGH_PRECISION
	float precisionSafeLength( vec3 v ) { return length( v ); }
#else
	float precisionSafeLength( vec3 v ) {
		float maxComponent = max3( abs( v ) );
		return length( v / maxComponent ) * maxComponent;
	}
#endif
struct IncidentLight {
	vec3 color;
	vec3 direction;
	bool visible;
};
struct ReflectedLight {
	vec3 directDiffuse;
	vec3 directSpecular;
	vec3 indirectDiffuse;
	vec3 indirectSpecular;
};
#ifdef USE_ALPHAHASH
	varying vec3 vPosition;
#endif
vec3 transformDirection( in vec3 dir, in mat4 matrix ) {
	return normalize( ( matrix * vec4( dir, 0.0 ) ).xyz );
}
vec3 inverseTransformDirection( in vec3 dir, in mat4 matrix ) {
	return normalize( ( vec4( dir, 0.0 ) * matrix ).xyz );
}
bool isPerspectiveMatrix( mat4 m ) {
	return m[ 2 ][ 3 ] == - 1.0;
}
vec2 equirectUv( in vec3 dir ) {
	float u = atan( dir.z, dir.x ) * RECIPROCAL_PI2 + 0.5;
	float v = asin( clamp( dir.y, - 1.0, 1.0 ) ) * RECIPROCAL_PI + 0.5;
	return vec2( u, v );
}
vec3 BRDF_Lambert( const in vec3 diffuseColor ) {
	return RECIPROCAL_PI * diffuseColor;
}
vec3 F_Schlick( const in vec3 f0, const in float f90, const in float dotVH ) {
	float fresnel = exp2( ( - 5.55473 * dotVH - 6.98316 ) * dotVH );
	return f0 * ( 1.0 - fresnel ) + ( f90 * fresnel );
}
float F_Schlick( const in float f0, const in float f90, const in float dotVH ) {
	float fresnel = exp2( ( - 5.55473 * dotVH - 6.98316 ) * dotVH );
	return f0 * ( 1.0 - fresnel ) + ( f90 * fresnel );
} // validated`,Yb=`#ifdef ENVMAP_TYPE_CUBE_UV
	#define cubeUV_minMipLevel 4.0
	#define cubeUV_minTileSize 16.0
	float getFace( vec3 direction ) {
		vec3 absDirection = abs( direction );
		float face = - 1.0;
		if ( absDirection.x > absDirection.z ) {
			if ( absDirection.x > absDirection.y )
				face = direction.x > 0.0 ? 0.0 : 3.0;
			else
				face = direction.y > 0.0 ? 1.0 : 4.0;
		} else {
			if ( absDirection.z > absDirection.y )
				face = direction.z > 0.0 ? 2.0 : 5.0;
			else
				face = direction.y > 0.0 ? 1.0 : 4.0;
		}
		return face;
	}
	vec2 getUV( vec3 direction, float face ) {
		vec2 uv;
		if ( face == 0.0 ) {
			uv = vec2( direction.z, direction.y ) / abs( direction.x );
		} else if ( face == 1.0 ) {
			uv = vec2( - direction.x, - direction.z ) / abs( direction.y );
		} else if ( face == 2.0 ) {
			uv = vec2( - direction.x, direction.y ) / abs( direction.z );
		} else if ( face == 3.0 ) {
			uv = vec2( - direction.z, direction.y ) / abs( direction.x );
		} else if ( face == 4.0 ) {
			uv = vec2( - direction.x, direction.z ) / abs( direction.y );
		} else {
			uv = vec2( direction.x, direction.y ) / abs( direction.z );
		}
		return 0.5 * ( uv + 1.0 );
	}
	vec3 bilinearCubeUV( sampler2D envMap, vec3 direction, float mipInt ) {
		float face = getFace( direction );
		float filterInt = max( cubeUV_minMipLevel - mipInt, 0.0 );
		mipInt = max( mipInt, cubeUV_minMipLevel );
		float faceSize = exp2( mipInt );
		highp vec2 uv = getUV( direction, face ) * ( faceSize - 2.0 ) + 1.0;
		if ( face > 2.0 ) {
			uv.y += faceSize;
			face -= 3.0;
		}
		uv.x += face * faceSize;
		uv.x += filterInt * 3.0 * cubeUV_minTileSize;
		uv.y += 4.0 * ( exp2( CUBEUV_MAX_MIP ) - faceSize );
		uv.x *= CUBEUV_TEXEL_WIDTH;
		uv.y *= CUBEUV_TEXEL_HEIGHT;
		#ifdef texture2DGradEXT
			return texture2DGradEXT( envMap, uv, vec2( 0.0 ), vec2( 0.0 ) ).rgb;
		#else
			return texture2D( envMap, uv ).rgb;
		#endif
	}
	#define cubeUV_r0 1.0
	#define cubeUV_m0 - 2.0
	#define cubeUV_r1 0.8
	#define cubeUV_m1 - 1.0
	#define cubeUV_r4 0.4
	#define cubeUV_m4 2.0
	#define cubeUV_r5 0.305
	#define cubeUV_m5 3.0
	#define cubeUV_r6 0.21
	#define cubeUV_m6 4.0
	float roughnessToMip( float roughness ) {
		float mip = 0.0;
		if ( roughness >= cubeUV_r1 ) {
			mip = ( cubeUV_r0 - roughness ) * ( cubeUV_m1 - cubeUV_m0 ) / ( cubeUV_r0 - cubeUV_r1 ) + cubeUV_m0;
		} else if ( roughness >= cubeUV_r4 ) {
			mip = ( cubeUV_r1 - roughness ) * ( cubeUV_m4 - cubeUV_m1 ) / ( cubeUV_r1 - cubeUV_r4 ) + cubeUV_m1;
		} else if ( roughness >= cubeUV_r5 ) {
			mip = ( cubeUV_r4 - roughness ) * ( cubeUV_m5 - cubeUV_m4 ) / ( cubeUV_r4 - cubeUV_r5 ) + cubeUV_m4;
		} else if ( roughness >= cubeUV_r6 ) {
			mip = ( cubeUV_r5 - roughness ) * ( cubeUV_m6 - cubeUV_m5 ) / ( cubeUV_r5 - cubeUV_r6 ) + cubeUV_m5;
		} else {
			mip = - 2.0 * log2( 1.16 * roughness );		}
		return mip;
	}
	vec4 textureCubeUV( sampler2D envMap, vec3 sampleDir, float roughness ) {
		float mip = clamp( roughnessToMip( roughness ), cubeUV_m0, CUBEUV_MAX_MIP );
		float mipF = fract( mip );
		float mipInt = floor( mip );
		vec3 color0 = bilinearCubeUV( envMap, sampleDir, mipInt );
		if ( mipF == 0.0 ) {
			return vec4( color0, 1.0 );
		} else {
			vec3 color1 = bilinearCubeUV( envMap, sampleDir, mipInt + 1.0 );
			return vec4( mix( color0, color1, mipF ), 1.0 );
		}
	}
#endif`,Zb=`vec3 transformedNormal = objectNormal;
#ifdef USE_TANGENT
	vec3 transformedTangent = objectTangent;
#endif
#ifdef USE_BATCHING
	mat3 bm = mat3( batchingMatrix );
	transformedNormal /= vec3( dot( bm[ 0 ], bm[ 0 ] ), dot( bm[ 1 ], bm[ 1 ] ), dot( bm[ 2 ], bm[ 2 ] ) );
	transformedNormal = bm * transformedNormal;
	#ifdef USE_TANGENT
		transformedTangent = bm * transformedTangent;
	#endif
#endif
#ifdef USE_INSTANCING
	mat3 im = mat3( instanceMatrix );
	transformedNormal /= vec3( dot( im[ 0 ], im[ 0 ] ), dot( im[ 1 ], im[ 1 ] ), dot( im[ 2 ], im[ 2 ] ) );
	transformedNormal = im * transformedNormal;
	#ifdef USE_TANGENT
		transformedTangent = im * transformedTangent;
	#endif
#endif
transformedNormal = normalMatrix * transformedNormal;
#ifdef FLIP_SIDED
	transformedNormal = - transformedNormal;
#endif
#ifdef USE_TANGENT
	transformedTangent = ( modelViewMatrix * vec4( transformedTangent, 0.0 ) ).xyz;
	#ifdef FLIP_SIDED
		transformedTangent = - transformedTangent;
	#endif
#endif`,Kb=`#ifdef USE_DISPLACEMENTMAP
	uniform sampler2D displacementMap;
	uniform float displacementScale;
	uniform float displacementBias;
#endif`,Qb=`#ifdef USE_DISPLACEMENTMAP
	transformed += normalize( objectNormal ) * ( texture2D( displacementMap, vDisplacementMapUv ).x * displacementScale + displacementBias );
#endif`,Jb=`#ifdef USE_EMISSIVEMAP
	vec4 emissiveColor = texture2D( emissiveMap, vEmissiveMapUv );
	#ifdef DECODE_VIDEO_TEXTURE_EMISSIVE
		emissiveColor = sRGBTransferEOTF( emissiveColor );
	#endif
	totalEmissiveRadiance *= emissiveColor.rgb;
#endif`,$b=`#ifdef USE_EMISSIVEMAP
	uniform sampler2D emissiveMap;
#endif`,eM="gl_FragColor = linearToOutputTexel( gl_FragColor );",tM=`vec4 LinearTransferOETF( in vec4 value ) {
	return value;
}
vec4 sRGBTransferEOTF( in vec4 value ) {
	return vec4( mix( pow( value.rgb * 0.9478672986 + vec3( 0.0521327014 ), vec3( 2.4 ) ), value.rgb * 0.0773993808, vec3( lessThanEqual( value.rgb, vec3( 0.04045 ) ) ) ), value.a );
}
vec4 sRGBTransferOETF( in vec4 value ) {
	return vec4( mix( pow( value.rgb, vec3( 0.41666 ) ) * 1.055 - vec3( 0.055 ), value.rgb * 12.92, vec3( lessThanEqual( value.rgb, vec3( 0.0031308 ) ) ) ), value.a );
}`,nM=`#ifdef USE_ENVMAP
	#ifdef ENV_WORLDPOS
		vec3 cameraToFrag;
		if ( isOrthographic ) {
			cameraToFrag = normalize( vec3( - viewMatrix[ 0 ][ 2 ], - viewMatrix[ 1 ][ 2 ], - viewMatrix[ 2 ][ 2 ] ) );
		} else {
			cameraToFrag = normalize( vWorldPosition - cameraPosition );
		}
		vec3 worldNormal = inverseTransformDirection( normal, viewMatrix );
		#ifdef ENVMAP_MODE_REFLECTION
			vec3 reflectVec = reflect( cameraToFrag, worldNormal );
		#else
			vec3 reflectVec = refract( cameraToFrag, worldNormal, refractionRatio );
		#endif
	#else
		vec3 reflectVec = vReflect;
	#endif
	#ifdef ENVMAP_TYPE_CUBE
		vec4 envColor = textureCube( envMap, envMapRotation * vec3( flipEnvMap * reflectVec.x, reflectVec.yz ) );
		#ifdef ENVMAP_BLENDING_MULTIPLY
			outgoingLight = mix( outgoingLight, outgoingLight * envColor.xyz, specularStrength * reflectivity );
		#elif defined( ENVMAP_BLENDING_MIX )
			outgoingLight = mix( outgoingLight, envColor.xyz, specularStrength * reflectivity );
		#elif defined( ENVMAP_BLENDING_ADD )
			outgoingLight += envColor.xyz * specularStrength * reflectivity;
		#endif
	#endif
#endif`,iM=`#ifdef USE_ENVMAP
	uniform float envMapIntensity;
	uniform float flipEnvMap;
	uniform mat3 envMapRotation;
	#ifdef ENVMAP_TYPE_CUBE
		uniform samplerCube envMap;
	#else
		uniform sampler2D envMap;
	#endif
#endif`,aM=`#ifdef USE_ENVMAP
	uniform float reflectivity;
	#if defined( USE_BUMPMAP ) || defined( USE_NORMALMAP ) || defined( PHONG ) || defined( LAMBERT )
		#define ENV_WORLDPOS
	#endif
	#ifdef ENV_WORLDPOS
		varying vec3 vWorldPosition;
		uniform float refractionRatio;
	#else
		varying vec3 vReflect;
	#endif
#endif`,sM=`#ifdef USE_ENVMAP
	#if defined( USE_BUMPMAP ) || defined( USE_NORMALMAP ) || defined( PHONG ) || defined( LAMBERT )
		#define ENV_WORLDPOS
	#endif
	#ifdef ENV_WORLDPOS
		
		varying vec3 vWorldPosition;
	#else
		varying vec3 vReflect;
		uniform float refractionRatio;
	#endif
#endif`,rM=`#ifdef USE_ENVMAP
	#ifdef ENV_WORLDPOS
		vWorldPosition = worldPosition.xyz;
	#else
		vec3 cameraToVertex;
		if ( isOrthographic ) {
			cameraToVertex = normalize( vec3( - viewMatrix[ 0 ][ 2 ], - viewMatrix[ 1 ][ 2 ], - viewMatrix[ 2 ][ 2 ] ) );
		} else {
			cameraToVertex = normalize( worldPosition.xyz - cameraPosition );
		}
		vec3 worldNormal = inverseTransformDirection( transformedNormal, viewMatrix );
		#ifdef ENVMAP_MODE_REFLECTION
			vReflect = reflect( cameraToVertex, worldNormal );
		#else
			vReflect = refract( cameraToVertex, worldNormal, refractionRatio );
		#endif
	#endif
#endif`,oM=`#ifdef USE_FOG
	vFogDepth = - mvPosition.z;
#endif`,lM=`#ifdef USE_FOG
	varying float vFogDepth;
#endif`,cM=`#ifdef USE_FOG
	#ifdef FOG_EXP2
		float fogFactor = 1.0 - exp( - fogDensity * fogDensity * vFogDepth * vFogDepth );
	#else
		float fogFactor = smoothstep( fogNear, fogFar, vFogDepth );
	#endif
	gl_FragColor.rgb = mix( gl_FragColor.rgb, fogColor, fogFactor );
#endif`,uM=`#ifdef USE_FOG
	uniform vec3 fogColor;
	varying float vFogDepth;
	#ifdef FOG_EXP2
		uniform float fogDensity;
	#else
		uniform float fogNear;
		uniform float fogFar;
	#endif
#endif`,fM=`#ifdef USE_GRADIENTMAP
	uniform sampler2D gradientMap;
#endif
vec3 getGradientIrradiance( vec3 normal, vec3 lightDirection ) {
	float dotNL = dot( normal, lightDirection );
	vec2 coord = vec2( dotNL * 0.5 + 0.5, 0.0 );
	#ifdef USE_GRADIENTMAP
		return vec3( texture2D( gradientMap, coord ).r );
	#else
		vec2 fw = fwidth( coord ) * 0.5;
		return mix( vec3( 0.7 ), vec3( 1.0 ), smoothstep( 0.7 - fw.x, 0.7 + fw.x, coord.x ) );
	#endif
}`,dM=`#ifdef USE_LIGHTMAP
	uniform sampler2D lightMap;
	uniform float lightMapIntensity;
#endif`,hM=`LambertMaterial material;
material.diffuseColor = diffuseColor.rgb;
material.specularStrength = specularStrength;`,pM=`varying vec3 vViewPosition;
struct LambertMaterial {
	vec3 diffuseColor;
	float specularStrength;
};
void RE_Direct_Lambert( const in IncidentLight directLight, const in vec3 geometryPosition, const in vec3 geometryNormal, const in vec3 geometryViewDir, const in vec3 geometryClearcoatNormal, const in LambertMaterial material, inout ReflectedLight reflectedLight ) {
	float dotNL = saturate( dot( geometryNormal, directLight.direction ) );
	vec3 irradiance = dotNL * directLight.color;
	reflectedLight.directDiffuse += irradiance * BRDF_Lambert( material.diffuseColor );
}
void RE_IndirectDiffuse_Lambert( const in vec3 irradiance, const in vec3 geometryPosition, const in vec3 geometryNormal, const in vec3 geometryViewDir, const in vec3 geometryClearcoatNormal, const in LambertMaterial material, inout ReflectedLight reflectedLight ) {
	reflectedLight.indirectDiffuse += irradiance * BRDF_Lambert( material.diffuseColor );
}
#define RE_Direct				RE_Direct_Lambert
#define RE_IndirectDiffuse		RE_IndirectDiffuse_Lambert`,mM=`uniform bool receiveShadow;
uniform vec3 ambientLightColor;
#if defined( USE_LIGHT_PROBES )
	uniform vec3 lightProbe[ 9 ];
#endif
vec3 shGetIrradianceAt( in vec3 normal, in vec3 shCoefficients[ 9 ] ) {
	float x = normal.x, y = normal.y, z = normal.z;
	vec3 result = shCoefficients[ 0 ] * 0.886227;
	result += shCoefficients[ 1 ] * 2.0 * 0.511664 * y;
	result += shCoefficients[ 2 ] * 2.0 * 0.511664 * z;
	result += shCoefficients[ 3 ] * 2.0 * 0.511664 * x;
	result += shCoefficients[ 4 ] * 2.0 * 0.429043 * x * y;
	result += shCoefficients[ 5 ] * 2.0 * 0.429043 * y * z;
	result += shCoefficients[ 6 ] * ( 0.743125 * z * z - 0.247708 );
	result += shCoefficients[ 7 ] * 2.0 * 0.429043 * x * z;
	result += shCoefficients[ 8 ] * 0.429043 * ( x * x - y * y );
	return result;
}
vec3 getLightProbeIrradiance( const in vec3 lightProbe[ 9 ], const in vec3 normal ) {
	vec3 worldNormal = inverseTransformDirection( normal, viewMatrix );
	vec3 irradiance = shGetIrradianceAt( worldNormal, lightProbe );
	return irradiance;
}
vec3 getAmbientLightIrradiance( const in vec3 ambientLightColor ) {
	vec3 irradiance = ambientLightColor;
	return irradiance;
}
float getDistanceAttenuation( const in float lightDistance, const in float cutoffDistance, const in float decayExponent ) {
	float distanceFalloff = 1.0 / max( pow( lightDistance, decayExponent ), 0.01 );
	if ( cutoffDistance > 0.0 ) {
		distanceFalloff *= pow2( saturate( 1.0 - pow4( lightDistance / cutoffDistance ) ) );
	}
	return distanceFalloff;
}
float getSpotAttenuation( const in float coneCosine, const in float penumbraCosine, const in float angleCosine ) {
	return smoothstep( coneCosine, penumbraCosine, angleCosine );
}
#if NUM_DIR_LIGHTS > 0
	struct DirectionalLight {
		vec3 direction;
		vec3 color;
	};
	uniform DirectionalLight directionalLights[ NUM_DIR_LIGHTS ];
	void getDirectionalLightInfo( const in DirectionalLight directionalLight, out IncidentLight light ) {
		light.color = directionalLight.color;
		light.direction = directionalLight.direction;
		light.visible = true;
	}
#endif
#if NUM_POINT_LIGHTS > 0
	struct PointLight {
		vec3 position;
		vec3 color;
		float distance;
		float decay;
	};
	uniform PointLight pointLights[ NUM_POINT_LIGHTS ];
	void getPointLightInfo( const in PointLight pointLight, const in vec3 geometryPosition, out IncidentLight light ) {
		vec3 lVector = pointLight.position - geometryPosition;
		light.direction = normalize( lVector );
		float lightDistance = length( lVector );
		light.color = pointLight.color;
		light.color *= getDistanceAttenuation( lightDistance, pointLight.distance, pointLight.decay );
		light.visible = ( light.color != vec3( 0.0 ) );
	}
#endif
#if NUM_SPOT_LIGHTS > 0
	struct SpotLight {
		vec3 position;
		vec3 direction;
		vec3 color;
		float distance;
		float decay;
		float coneCos;
		float penumbraCos;
	};
	uniform SpotLight spotLights[ NUM_SPOT_LIGHTS ];
	void getSpotLightInfo( const in SpotLight spotLight, const in vec3 geometryPosition, out IncidentLight light ) {
		vec3 lVector = spotLight.position - geometryPosition;
		light.direction = normalize( lVector );
		float angleCos = dot( light.direction, spotLight.direction );
		float spotAttenuation = getSpotAttenuation( spotLight.coneCos, spotLight.penumbraCos, angleCos );
		if ( spotAttenuation > 0.0 ) {
			float lightDistance = length( lVector );
			light.color = spotLight.color * spotAttenuation;
			light.color *= getDistanceAttenuation( lightDistance, spotLight.distance, spotLight.decay );
			light.visible = ( light.color != vec3( 0.0 ) );
		} else {
			light.color = vec3( 0.0 );
			light.visible = false;
		}
	}
#endif
#if NUM_RECT_AREA_LIGHTS > 0
	struct RectAreaLight {
		vec3 color;
		vec3 position;
		vec3 halfWidth;
		vec3 halfHeight;
	};
	uniform sampler2D ltc_1;	uniform sampler2D ltc_2;
	uniform RectAreaLight rectAreaLights[ NUM_RECT_AREA_LIGHTS ];
#endif
#if NUM_HEMI_LIGHTS > 0
	struct HemisphereLight {
		vec3 direction;
		vec3 skyColor;
		vec3 groundColor;
	};
	uniform HemisphereLight hemisphereLights[ NUM_HEMI_LIGHTS ];
	vec3 getHemisphereLightIrradiance( const in HemisphereLight hemiLight, const in vec3 normal ) {
		float dotNL = dot( normal, hemiLight.direction );
		float hemiDiffuseWeight = 0.5 * dotNL + 0.5;
		vec3 irradiance = mix( hemiLight.groundColor, hemiLight.skyColor, hemiDiffuseWeight );
		return irradiance;
	}
#endif`,gM=`#ifdef USE_ENVMAP
	vec3 getIBLIrradiance( const in vec3 normal ) {
		#ifdef ENVMAP_TYPE_CUBE_UV
			vec3 worldNormal = inverseTransformDirection( normal, viewMatrix );
			vec4 envMapColor = textureCubeUV( envMap, envMapRotation * worldNormal, 1.0 );
			return PI * envMapColor.rgb * envMapIntensity;
		#else
			return vec3( 0.0 );
		#endif
	}
	vec3 getIBLRadiance( const in vec3 viewDir, const in vec3 normal, const in float roughness ) {
		#ifdef ENVMAP_TYPE_CUBE_UV
			vec3 reflectVec = reflect( - viewDir, normal );
			reflectVec = normalize( mix( reflectVec, normal, pow4( roughness ) ) );
			reflectVec = inverseTransformDirection( reflectVec, viewMatrix );
			vec4 envMapColor = textureCubeUV( envMap, envMapRotation * reflectVec, roughness );
			return envMapColor.rgb * envMapIntensity;
		#else
			return vec3( 0.0 );
		#endif
	}
	#ifdef USE_ANISOTROPY
		vec3 getIBLAnisotropyRadiance( const in vec3 viewDir, const in vec3 normal, const in float roughness, const in vec3 bitangent, const in float anisotropy ) {
			#ifdef ENVMAP_TYPE_CUBE_UV
				vec3 bentNormal = cross( bitangent, viewDir );
				bentNormal = normalize( cross( bentNormal, bitangent ) );
				bentNormal = normalize( mix( bentNormal, normal, pow2( pow2( 1.0 - anisotropy * ( 1.0 - roughness ) ) ) ) );
				return getIBLRadiance( viewDir, bentNormal, roughness );
			#else
				return vec3( 0.0 );
			#endif
		}
	#endif
#endif`,_M=`ToonMaterial material;
material.diffuseColor = diffuseColor.rgb;`,vM=`varying vec3 vViewPosition;
struct ToonMaterial {
	vec3 diffuseColor;
};
void RE_Direct_Toon( const in IncidentLight directLight, const in vec3 geometryPosition, const in vec3 geometryNormal, const in vec3 geometryViewDir, const in vec3 geometryClearcoatNormal, const in ToonMaterial material, inout ReflectedLight reflectedLight ) {
	vec3 irradiance = getGradientIrradiance( geometryNormal, directLight.direction ) * directLight.color;
	reflectedLight.directDiffuse += irradiance * BRDF_Lambert( material.diffuseColor );
}
void RE_IndirectDiffuse_Toon( const in vec3 irradiance, const in vec3 geometryPosition, const in vec3 geometryNormal, const in vec3 geometryViewDir, const in vec3 geometryClearcoatNormal, const in ToonMaterial material, inout ReflectedLight reflectedLight ) {
	reflectedLight.indirectDiffuse += irradiance * BRDF_Lambert( material.diffuseColor );
}
#define RE_Direct				RE_Direct_Toon
#define RE_IndirectDiffuse		RE_IndirectDiffuse_Toon`,xM=`BlinnPhongMaterial material;
material.diffuseColor = diffuseColor.rgb;
material.specularColor = specular;
material.specularShininess = shininess;
material.specularStrength = specularStrength;`,yM=`varying vec3 vViewPosition;
struct BlinnPhongMaterial {
	vec3 diffuseColor;
	vec3 specularColor;
	float specularShininess;
	float specularStrength;
};
void RE_Direct_BlinnPhong( const in IncidentLight directLight, const in vec3 geometryPosition, const in vec3 geometryNormal, const in vec3 geometryViewDir, const in vec3 geometryClearcoatNormal, const in BlinnPhongMaterial material, inout ReflectedLight reflectedLight ) {
	float dotNL = saturate( dot( geometryNormal, directLight.direction ) );
	vec3 irradiance = dotNL * directLight.color;
	reflectedLight.directDiffuse += irradiance * BRDF_Lambert( material.diffuseColor );
	reflectedLight.directSpecular += irradiance * BRDF_BlinnPhong( directLight.direction, geometryViewDir, geometryNormal, material.specularColor, material.specularShininess ) * material.specularStrength;
}
void RE_IndirectDiffuse_BlinnPhong( const in vec3 irradiance, const in vec3 geometryPosition, const in vec3 geometryNormal, const in vec3 geometryViewDir, const in vec3 geometryClearcoatNormal, const in BlinnPhongMaterial material, inout ReflectedLight reflectedLight ) {
	reflectedLight.indirectDiffuse += irradiance * BRDF_Lambert( material.diffuseColor );
}
#define RE_Direct				RE_Direct_BlinnPhong
#define RE_IndirectDiffuse		RE_IndirectDiffuse_BlinnPhong`,SM=`PhysicalMaterial material;
material.diffuseColor = diffuseColor.rgb;
material.diffuseContribution = diffuseColor.rgb * ( 1.0 - metalnessFactor );
material.metalness = metalnessFactor;
vec3 dxy = max( abs( dFdx( nonPerturbedNormal ) ), abs( dFdy( nonPerturbedNormal ) ) );
float geometryRoughness = max( max( dxy.x, dxy.y ), dxy.z );
material.roughness = max( roughnessFactor, 0.0525 );material.roughness += geometryRoughness;
material.roughness = min( material.roughness, 1.0 );
#ifdef IOR
	material.ior = ior;
	#ifdef USE_SPECULAR
		float specularIntensityFactor = specularIntensity;
		vec3 specularColorFactor = specularColor;
		#ifdef USE_SPECULAR_COLORMAP
			specularColorFactor *= texture2D( specularColorMap, vSpecularColorMapUv ).rgb;
		#endif
		#ifdef USE_SPECULAR_INTENSITYMAP
			specularIntensityFactor *= texture2D( specularIntensityMap, vSpecularIntensityMapUv ).a;
		#endif
		material.specularF90 = mix( specularIntensityFactor, 1.0, metalnessFactor );
	#else
		float specularIntensityFactor = 1.0;
		vec3 specularColorFactor = vec3( 1.0 );
		material.specularF90 = 1.0;
	#endif
	material.specularColor = min( pow2( ( material.ior - 1.0 ) / ( material.ior + 1.0 ) ) * specularColorFactor, vec3( 1.0 ) ) * specularIntensityFactor;
	material.specularColorBlended = mix( material.specularColor, diffuseColor.rgb, metalnessFactor );
#else
	material.specularColor = vec3( 0.04 );
	material.specularColorBlended = mix( material.specularColor, diffuseColor.rgb, metalnessFactor );
	material.specularF90 = 1.0;
#endif
#ifdef USE_CLEARCOAT
	material.clearcoat = clearcoat;
	material.clearcoatRoughness = clearcoatRoughness;
	material.clearcoatF0 = vec3( 0.04 );
	material.clearcoatF90 = 1.0;
	#ifdef USE_CLEARCOATMAP
		material.clearcoat *= texture2D( clearcoatMap, vClearcoatMapUv ).x;
	#endif
	#ifdef USE_CLEARCOAT_ROUGHNESSMAP
		material.clearcoatRoughness *= texture2D( clearcoatRoughnessMap, vClearcoatRoughnessMapUv ).y;
	#endif
	material.clearcoat = saturate( material.clearcoat );	material.clearcoatRoughness = max( material.clearcoatRoughness, 0.0525 );
	material.clearcoatRoughness += geometryRoughness;
	material.clearcoatRoughness = min( material.clearcoatRoughness, 1.0 );
#endif
#ifdef USE_DISPERSION
	material.dispersion = dispersion;
#endif
#ifdef USE_IRIDESCENCE
	material.iridescence = iridescence;
	material.iridescenceIOR = iridescenceIOR;
	#ifdef USE_IRIDESCENCEMAP
		material.iridescence *= texture2D( iridescenceMap, vIridescenceMapUv ).r;
	#endif
	#ifdef USE_IRIDESCENCE_THICKNESSMAP
		material.iridescenceThickness = (iridescenceThicknessMaximum - iridescenceThicknessMinimum) * texture2D( iridescenceThicknessMap, vIridescenceThicknessMapUv ).g + iridescenceThicknessMinimum;
	#else
		material.iridescenceThickness = iridescenceThicknessMaximum;
	#endif
#endif
#ifdef USE_SHEEN
	material.sheenColor = sheenColor;
	#ifdef USE_SHEEN_COLORMAP
		material.sheenColor *= texture2D( sheenColorMap, vSheenColorMapUv ).rgb;
	#endif
	material.sheenRoughness = clamp( sheenRoughness, 0.0001, 1.0 );
	#ifdef USE_SHEEN_ROUGHNESSMAP
		material.sheenRoughness *= texture2D( sheenRoughnessMap, vSheenRoughnessMapUv ).a;
	#endif
#endif
#ifdef USE_ANISOTROPY
	#ifdef USE_ANISOTROPYMAP
		mat2 anisotropyMat = mat2( anisotropyVector.x, anisotropyVector.y, - anisotropyVector.y, anisotropyVector.x );
		vec3 anisotropyPolar = texture2D( anisotropyMap, vAnisotropyMapUv ).rgb;
		vec2 anisotropyV = anisotropyMat * normalize( 2.0 * anisotropyPolar.rg - vec2( 1.0 ) ) * anisotropyPolar.b;
	#else
		vec2 anisotropyV = anisotropyVector;
	#endif
	material.anisotropy = length( anisotropyV );
	if( material.anisotropy == 0.0 ) {
		anisotropyV = vec2( 1.0, 0.0 );
	} else {
		anisotropyV /= material.anisotropy;
		material.anisotropy = saturate( material.anisotropy );
	}
	material.alphaT = mix( pow2( material.roughness ), 1.0, pow2( material.anisotropy ) );
	material.anisotropyT = tbn[ 0 ] * anisotropyV.x + tbn[ 1 ] * anisotropyV.y;
	material.anisotropyB = tbn[ 1 ] * anisotropyV.x - tbn[ 0 ] * anisotropyV.y;
#endif`,bM=`uniform sampler2D dfgLUT;
struct PhysicalMaterial {
	vec3 diffuseColor;
	vec3 diffuseContribution;
	vec3 specularColor;
	vec3 specularColorBlended;
	float roughness;
	float metalness;
	float specularF90;
	float dispersion;
	#ifdef USE_CLEARCOAT
		float clearcoat;
		float clearcoatRoughness;
		vec3 clearcoatF0;
		float clearcoatF90;
	#endif
	#ifdef USE_IRIDESCENCE
		float iridescence;
		float iridescenceIOR;
		float iridescenceThickness;
		vec3 iridescenceFresnel;
		vec3 iridescenceF0;
		vec3 iridescenceFresnelDielectric;
		vec3 iridescenceFresnelMetallic;
	#endif
	#ifdef USE_SHEEN
		vec3 sheenColor;
		float sheenRoughness;
	#endif
	#ifdef IOR
		float ior;
	#endif
	#ifdef USE_TRANSMISSION
		float transmission;
		float transmissionAlpha;
		float thickness;
		float attenuationDistance;
		vec3 attenuationColor;
	#endif
	#ifdef USE_ANISOTROPY
		float anisotropy;
		float alphaT;
		vec3 anisotropyT;
		vec3 anisotropyB;
	#endif
};
vec3 clearcoatSpecularDirect = vec3( 0.0 );
vec3 clearcoatSpecularIndirect = vec3( 0.0 );
vec3 sheenSpecularDirect = vec3( 0.0 );
vec3 sheenSpecularIndirect = vec3(0.0 );
vec3 Schlick_to_F0( const in vec3 f, const in float f90, const in float dotVH ) {
    float x = clamp( 1.0 - dotVH, 0.0, 1.0 );
    float x2 = x * x;
    float x5 = clamp( x * x2 * x2, 0.0, 0.9999 );
    return ( f - vec3( f90 ) * x5 ) / ( 1.0 - x5 );
}
float V_GGX_SmithCorrelated( const in float alpha, const in float dotNL, const in float dotNV ) {
	float a2 = pow2( alpha );
	float gv = dotNL * sqrt( a2 + ( 1.0 - a2 ) * pow2( dotNV ) );
	float gl = dotNV * sqrt( a2 + ( 1.0 - a2 ) * pow2( dotNL ) );
	return 0.5 / max( gv + gl, EPSILON );
}
float D_GGX( const in float alpha, const in float dotNH ) {
	float a2 = pow2( alpha );
	float denom = pow2( dotNH ) * ( a2 - 1.0 ) + 1.0;
	return RECIPROCAL_PI * a2 / pow2( denom );
}
#ifdef USE_ANISOTROPY
	float V_GGX_SmithCorrelated_Anisotropic( const in float alphaT, const in float alphaB, const in float dotTV, const in float dotBV, const in float dotTL, const in float dotBL, const in float dotNV, const in float dotNL ) {
		float gv = dotNL * length( vec3( alphaT * dotTV, alphaB * dotBV, dotNV ) );
		float gl = dotNV * length( vec3( alphaT * dotTL, alphaB * dotBL, dotNL ) );
		float v = 0.5 / ( gv + gl );
		return v;
	}
	float D_GGX_Anisotropic( const in float alphaT, const in float alphaB, const in float dotNH, const in float dotTH, const in float dotBH ) {
		float a2 = alphaT * alphaB;
		highp vec3 v = vec3( alphaB * dotTH, alphaT * dotBH, a2 * dotNH );
		highp float v2 = dot( v, v );
		float w2 = a2 / v2;
		return RECIPROCAL_PI * a2 * pow2 ( w2 );
	}
#endif
#ifdef USE_CLEARCOAT
	vec3 BRDF_GGX_Clearcoat( const in vec3 lightDir, const in vec3 viewDir, const in vec3 normal, const in PhysicalMaterial material) {
		vec3 f0 = material.clearcoatF0;
		float f90 = material.clearcoatF90;
		float roughness = material.clearcoatRoughness;
		float alpha = pow2( roughness );
		vec3 halfDir = normalize( lightDir + viewDir );
		float dotNL = saturate( dot( normal, lightDir ) );
		float dotNV = saturate( dot( normal, viewDir ) );
		float dotNH = saturate( dot( normal, halfDir ) );
		float dotVH = saturate( dot( viewDir, halfDir ) );
		vec3 F = F_Schlick( f0, f90, dotVH );
		float V = V_GGX_SmithCorrelated( alpha, dotNL, dotNV );
		float D = D_GGX( alpha, dotNH );
		return F * ( V * D );
	}
#endif
vec3 BRDF_GGX( const in vec3 lightDir, const in vec3 viewDir, const in vec3 normal, const in PhysicalMaterial material ) {
	vec3 f0 = material.specularColorBlended;
	float f90 = material.specularF90;
	float roughness = material.roughness;
	float alpha = pow2( roughness );
	vec3 halfDir = normalize( lightDir + viewDir );
	float dotNL = saturate( dot( normal, lightDir ) );
	float dotNV = saturate( dot( normal, viewDir ) );
	float dotNH = saturate( dot( normal, halfDir ) );
	float dotVH = saturate( dot( viewDir, halfDir ) );
	vec3 F = F_Schlick( f0, f90, dotVH );
	#ifdef USE_IRIDESCENCE
		F = mix( F, material.iridescenceFresnel, material.iridescence );
	#endif
	#ifdef USE_ANISOTROPY
		float dotTL = dot( material.anisotropyT, lightDir );
		float dotTV = dot( material.anisotropyT, viewDir );
		float dotTH = dot( material.anisotropyT, halfDir );
		float dotBL = dot( material.anisotropyB, lightDir );
		float dotBV = dot( material.anisotropyB, viewDir );
		float dotBH = dot( material.anisotropyB, halfDir );
		float V = V_GGX_SmithCorrelated_Anisotropic( material.alphaT, alpha, dotTV, dotBV, dotTL, dotBL, dotNV, dotNL );
		float D = D_GGX_Anisotropic( material.alphaT, alpha, dotNH, dotTH, dotBH );
	#else
		float V = V_GGX_SmithCorrelated( alpha, dotNL, dotNV );
		float D = D_GGX( alpha, dotNH );
	#endif
	return F * ( V * D );
}
vec2 LTC_Uv( const in vec3 N, const in vec3 V, const in float roughness ) {
	const float LUT_SIZE = 64.0;
	const float LUT_SCALE = ( LUT_SIZE - 1.0 ) / LUT_SIZE;
	const float LUT_BIAS = 0.5 / LUT_SIZE;
	float dotNV = saturate( dot( N, V ) );
	vec2 uv = vec2( roughness, sqrt( 1.0 - dotNV ) );
	uv = uv * LUT_SCALE + LUT_BIAS;
	return uv;
}
float LTC_ClippedSphereFormFactor( const in vec3 f ) {
	float l = length( f );
	return max( ( l * l + f.z ) / ( l + 1.0 ), 0.0 );
}
vec3 LTC_EdgeVectorFormFactor( const in vec3 v1, const in vec3 v2 ) {
	float x = dot( v1, v2 );
	float y = abs( x );
	float a = 0.8543985 + ( 0.4965155 + 0.0145206 * y ) * y;
	float b = 3.4175940 + ( 4.1616724 + y ) * y;
	float v = a / b;
	float theta_sintheta = ( x > 0.0 ) ? v : 0.5 * inversesqrt( max( 1.0 - x * x, 1e-7 ) ) - v;
	return cross( v1, v2 ) * theta_sintheta;
}
vec3 LTC_Evaluate( const in vec3 N, const in vec3 V, const in vec3 P, const in mat3 mInv, const in vec3 rectCoords[ 4 ] ) {
	vec3 v1 = rectCoords[ 1 ] - rectCoords[ 0 ];
	vec3 v2 = rectCoords[ 3 ] - rectCoords[ 0 ];
	vec3 lightNormal = cross( v1, v2 );
	if( dot( lightNormal, P - rectCoords[ 0 ] ) < 0.0 ) return vec3( 0.0 );
	vec3 T1, T2;
	T1 = normalize( V - N * dot( V, N ) );
	T2 = - cross( N, T1 );
	mat3 mat = mInv * transpose( mat3( T1, T2, N ) );
	vec3 coords[ 4 ];
	coords[ 0 ] = mat * ( rectCoords[ 0 ] - P );
	coords[ 1 ] = mat * ( rectCoords[ 1 ] - P );
	coords[ 2 ] = mat * ( rectCoords[ 2 ] - P );
	coords[ 3 ] = mat * ( rectCoords[ 3 ] - P );
	coords[ 0 ] = normalize( coords[ 0 ] );
	coords[ 1 ] = normalize( coords[ 1 ] );
	coords[ 2 ] = normalize( coords[ 2 ] );
	coords[ 3 ] = normalize( coords[ 3 ] );
	vec3 vectorFormFactor = vec3( 0.0 );
	vectorFormFactor += LTC_EdgeVectorFormFactor( coords[ 0 ], coords[ 1 ] );
	vectorFormFactor += LTC_EdgeVectorFormFactor( coords[ 1 ], coords[ 2 ] );
	vectorFormFactor += LTC_EdgeVectorFormFactor( coords[ 2 ], coords[ 3 ] );
	vectorFormFactor += LTC_EdgeVectorFormFactor( coords[ 3 ], coords[ 0 ] );
	float result = LTC_ClippedSphereFormFactor( vectorFormFactor );
	return vec3( result );
}
#if defined( USE_SHEEN )
float D_Charlie( float roughness, float dotNH ) {
	float alpha = pow2( roughness );
	float invAlpha = 1.0 / alpha;
	float cos2h = dotNH * dotNH;
	float sin2h = max( 1.0 - cos2h, 0.0078125 );
	return ( 2.0 + invAlpha ) * pow( sin2h, invAlpha * 0.5 ) / ( 2.0 * PI );
}
float V_Neubelt( float dotNV, float dotNL ) {
	return saturate( 1.0 / ( 4.0 * ( dotNL + dotNV - dotNL * dotNV ) ) );
}
vec3 BRDF_Sheen( const in vec3 lightDir, const in vec3 viewDir, const in vec3 normal, vec3 sheenColor, const in float sheenRoughness ) {
	vec3 halfDir = normalize( lightDir + viewDir );
	float dotNL = saturate( dot( normal, lightDir ) );
	float dotNV = saturate( dot( normal, viewDir ) );
	float dotNH = saturate( dot( normal, halfDir ) );
	float D = D_Charlie( sheenRoughness, dotNH );
	float V = V_Neubelt( dotNV, dotNL );
	return sheenColor * ( D * V );
}
#endif
float IBLSheenBRDF( const in vec3 normal, const in vec3 viewDir, const in float roughness ) {
	float dotNV = saturate( dot( normal, viewDir ) );
	float r2 = roughness * roughness;
	float rInv = 1.0 / ( roughness + 0.1 );
	float a = -1.9362 + 1.0678 * roughness + 0.4573 * r2 - 0.8469 * rInv;
	float b = -0.6014 + 0.5538 * roughness - 0.4670 * r2 - 0.1255 * rInv;
	float DG = exp( a * dotNV + b );
	return saturate( DG );
}
vec3 EnvironmentBRDF( const in vec3 normal, const in vec3 viewDir, const in vec3 specularColor, const in float specularF90, const in float roughness ) {
	float dotNV = saturate( dot( normal, viewDir ) );
	vec2 fab = texture2D( dfgLUT, vec2( roughness, dotNV ) ).rg;
	return specularColor * fab.x + specularF90 * fab.y;
}
#ifdef USE_IRIDESCENCE
void computeMultiscatteringIridescence( const in vec3 normal, const in vec3 viewDir, const in vec3 specularColor, const in float specularF90, const in float iridescence, const in vec3 iridescenceF0, const in float roughness, inout vec3 singleScatter, inout vec3 multiScatter ) {
#else
void computeMultiscattering( const in vec3 normal, const in vec3 viewDir, const in vec3 specularColor, const in float specularF90, const in float roughness, inout vec3 singleScatter, inout vec3 multiScatter ) {
#endif
	float dotNV = saturate( dot( normal, viewDir ) );
	vec2 fab = texture2D( dfgLUT, vec2( roughness, dotNV ) ).rg;
	#ifdef USE_IRIDESCENCE
		vec3 Fr = mix( specularColor, iridescenceF0, iridescence );
	#else
		vec3 Fr = specularColor;
	#endif
	vec3 FssEss = Fr * fab.x + specularF90 * fab.y;
	float Ess = fab.x + fab.y;
	float Ems = 1.0 - Ess;
	vec3 Favg = Fr + ( 1.0 - Fr ) * 0.047619;	vec3 Fms = FssEss * Favg / ( 1.0 - Ems * Favg );
	singleScatter += FssEss;
	multiScatter += Fms * Ems;
}
vec3 BRDF_GGX_Multiscatter( const in vec3 lightDir, const in vec3 viewDir, const in vec3 normal, const in PhysicalMaterial material ) {
	vec3 singleScatter = BRDF_GGX( lightDir, viewDir, normal, material );
	float dotNL = saturate( dot( normal, lightDir ) );
	float dotNV = saturate( dot( normal, viewDir ) );
	vec2 dfgV = texture2D( dfgLUT, vec2( material.roughness, dotNV ) ).rg;
	vec2 dfgL = texture2D( dfgLUT, vec2( material.roughness, dotNL ) ).rg;
	vec3 FssEss_V = material.specularColorBlended * dfgV.x + material.specularF90 * dfgV.y;
	vec3 FssEss_L = material.specularColorBlended * dfgL.x + material.specularF90 * dfgL.y;
	float Ess_V = dfgV.x + dfgV.y;
	float Ess_L = dfgL.x + dfgL.y;
	float Ems_V = 1.0 - Ess_V;
	float Ems_L = 1.0 - Ess_L;
	vec3 Favg = material.specularColorBlended + ( 1.0 - material.specularColorBlended ) * 0.047619;
	vec3 Fms = FssEss_V * FssEss_L * Favg / ( 1.0 - Ems_V * Ems_L * Favg + EPSILON );
	float compensationFactor = Ems_V * Ems_L;
	vec3 multiScatter = Fms * compensationFactor;
	return singleScatter + multiScatter;
}
#if NUM_RECT_AREA_LIGHTS > 0
	void RE_Direct_RectArea_Physical( const in RectAreaLight rectAreaLight, const in vec3 geometryPosition, const in vec3 geometryNormal, const in vec3 geometryViewDir, const in vec3 geometryClearcoatNormal, const in PhysicalMaterial material, inout ReflectedLight reflectedLight ) {
		vec3 normal = geometryNormal;
		vec3 viewDir = geometryViewDir;
		vec3 position = geometryPosition;
		vec3 lightPos = rectAreaLight.position;
		vec3 halfWidth = rectAreaLight.halfWidth;
		vec3 halfHeight = rectAreaLight.halfHeight;
		vec3 lightColor = rectAreaLight.color;
		float roughness = material.roughness;
		vec3 rectCoords[ 4 ];
		rectCoords[ 0 ] = lightPos + halfWidth - halfHeight;		rectCoords[ 1 ] = lightPos - halfWidth - halfHeight;
		rectCoords[ 2 ] = lightPos - halfWidth + halfHeight;
		rectCoords[ 3 ] = lightPos + halfWidth + halfHeight;
		vec2 uv = LTC_Uv( normal, viewDir, roughness );
		vec4 t1 = texture2D( ltc_1, uv );
		vec4 t2 = texture2D( ltc_2, uv );
		mat3 mInv = mat3(
			vec3( t1.x, 0, t1.y ),
			vec3(    0, 1,    0 ),
			vec3( t1.z, 0, t1.w )
		);
		vec3 fresnel = ( material.specularColorBlended * t2.x + ( material.specularF90 - material.specularColorBlended ) * t2.y );
		reflectedLight.directSpecular += lightColor * fresnel * LTC_Evaluate( normal, viewDir, position, mInv, rectCoords );
		reflectedLight.directDiffuse += lightColor * material.diffuseContribution * LTC_Evaluate( normal, viewDir, position, mat3( 1.0 ), rectCoords );
		#ifdef USE_CLEARCOAT
			vec3 Ncc = geometryClearcoatNormal;
			vec2 uvClearcoat = LTC_Uv( Ncc, viewDir, material.clearcoatRoughness );
			vec4 t1Clearcoat = texture2D( ltc_1, uvClearcoat );
			vec4 t2Clearcoat = texture2D( ltc_2, uvClearcoat );
			mat3 mInvClearcoat = mat3(
				vec3( t1Clearcoat.x, 0, t1Clearcoat.y ),
				vec3(             0, 1,             0 ),
				vec3( t1Clearcoat.z, 0, t1Clearcoat.w )
			);
			vec3 fresnelClearcoat = material.clearcoatF0 * t2Clearcoat.x + ( material.clearcoatF90 - material.clearcoatF0 ) * t2Clearcoat.y;
			clearcoatSpecularDirect += lightColor * fresnelClearcoat * LTC_Evaluate( Ncc, viewDir, position, mInvClearcoat, rectCoords );
		#endif
	}
#endif
void RE_Direct_Physical( const in IncidentLight directLight, const in vec3 geometryPosition, const in vec3 geometryNormal, const in vec3 geometryViewDir, const in vec3 geometryClearcoatNormal, const in PhysicalMaterial material, inout ReflectedLight reflectedLight ) {
	float dotNL = saturate( dot( geometryNormal, directLight.direction ) );
	vec3 irradiance = dotNL * directLight.color;
	#ifdef USE_CLEARCOAT
		float dotNLcc = saturate( dot( geometryClearcoatNormal, directLight.direction ) );
		vec3 ccIrradiance = dotNLcc * directLight.color;
		clearcoatSpecularDirect += ccIrradiance * BRDF_GGX_Clearcoat( directLight.direction, geometryViewDir, geometryClearcoatNormal, material );
	#endif
	#ifdef USE_SHEEN
 
 		sheenSpecularDirect += irradiance * BRDF_Sheen( directLight.direction, geometryViewDir, geometryNormal, material.sheenColor, material.sheenRoughness );
 
 		float sheenAlbedoV = IBLSheenBRDF( geometryNormal, geometryViewDir, material.sheenRoughness );
 		float sheenAlbedoL = IBLSheenBRDF( geometryNormal, directLight.direction, material.sheenRoughness );
 
 		float sheenEnergyComp = 1.0 - max3( material.sheenColor ) * max( sheenAlbedoV, sheenAlbedoL );
 
 		irradiance *= sheenEnergyComp;
 
 	#endif
	reflectedLight.directSpecular += irradiance * BRDF_GGX_Multiscatter( directLight.direction, geometryViewDir, geometryNormal, material );
	reflectedLight.directDiffuse += irradiance * BRDF_Lambert( material.diffuseContribution );
}
void RE_IndirectDiffuse_Physical( const in vec3 irradiance, const in vec3 geometryPosition, const in vec3 geometryNormal, const in vec3 geometryViewDir, const in vec3 geometryClearcoatNormal, const in PhysicalMaterial material, inout ReflectedLight reflectedLight ) {
	vec3 diffuse = irradiance * BRDF_Lambert( material.diffuseContribution );
	#ifdef USE_SHEEN
		float sheenAlbedo = IBLSheenBRDF( geometryNormal, geometryViewDir, material.sheenRoughness );
		float sheenEnergyComp = 1.0 - max3( material.sheenColor ) * sheenAlbedo;
		diffuse *= sheenEnergyComp;
	#endif
	reflectedLight.indirectDiffuse += diffuse;
}
void RE_IndirectSpecular_Physical( const in vec3 radiance, const in vec3 irradiance, const in vec3 clearcoatRadiance, const in vec3 geometryPosition, const in vec3 geometryNormal, const in vec3 geometryViewDir, const in vec3 geometryClearcoatNormal, const in PhysicalMaterial material, inout ReflectedLight reflectedLight) {
	#ifdef USE_CLEARCOAT
		clearcoatSpecularIndirect += clearcoatRadiance * EnvironmentBRDF( geometryClearcoatNormal, geometryViewDir, material.clearcoatF0, material.clearcoatF90, material.clearcoatRoughness );
	#endif
	#ifdef USE_SHEEN
		sheenSpecularIndirect += irradiance * material.sheenColor * IBLSheenBRDF( geometryNormal, geometryViewDir, material.sheenRoughness ) * RECIPROCAL_PI;
 	#endif
	vec3 singleScatteringDielectric = vec3( 0.0 );
	vec3 multiScatteringDielectric = vec3( 0.0 );
	vec3 singleScatteringMetallic = vec3( 0.0 );
	vec3 multiScatteringMetallic = vec3( 0.0 );
	#ifdef USE_IRIDESCENCE
		computeMultiscatteringIridescence( geometryNormal, geometryViewDir, material.specularColor, material.specularF90, material.iridescence, material.iridescenceFresnelDielectric, material.roughness, singleScatteringDielectric, multiScatteringDielectric );
		computeMultiscatteringIridescence( geometryNormal, geometryViewDir, material.diffuseColor, material.specularF90, material.iridescence, material.iridescenceFresnelMetallic, material.roughness, singleScatteringMetallic, multiScatteringMetallic );
	#else
		computeMultiscattering( geometryNormal, geometryViewDir, material.specularColor, material.specularF90, material.roughness, singleScatteringDielectric, multiScatteringDielectric );
		computeMultiscattering( geometryNormal, geometryViewDir, material.diffuseColor, material.specularF90, material.roughness, singleScatteringMetallic, multiScatteringMetallic );
	#endif
	vec3 singleScattering = mix( singleScatteringDielectric, singleScatteringMetallic, material.metalness );
	vec3 multiScattering = mix( multiScatteringDielectric, multiScatteringMetallic, material.metalness );
	vec3 totalScatteringDielectric = singleScatteringDielectric + multiScatteringDielectric;
	vec3 diffuse = material.diffuseContribution * ( 1.0 - totalScatteringDielectric );
	vec3 cosineWeightedIrradiance = irradiance * RECIPROCAL_PI;
	vec3 indirectSpecular = radiance * singleScattering;
	indirectSpecular += multiScattering * cosineWeightedIrradiance;
	vec3 indirectDiffuse = diffuse * cosineWeightedIrradiance;
	#ifdef USE_SHEEN
		float sheenAlbedo = IBLSheenBRDF( geometryNormal, geometryViewDir, material.sheenRoughness );
		float sheenEnergyComp = 1.0 - max3( material.sheenColor ) * sheenAlbedo;
		indirectSpecular *= sheenEnergyComp;
		indirectDiffuse *= sheenEnergyComp;
	#endif
	reflectedLight.indirectSpecular += indirectSpecular;
	reflectedLight.indirectDiffuse += indirectDiffuse;
}
#define RE_Direct				RE_Direct_Physical
#define RE_Direct_RectArea		RE_Direct_RectArea_Physical
#define RE_IndirectDiffuse		RE_IndirectDiffuse_Physical
#define RE_IndirectSpecular		RE_IndirectSpecular_Physical
float computeSpecularOcclusion( const in float dotNV, const in float ambientOcclusion, const in float roughness ) {
	return saturate( pow( dotNV + ambientOcclusion, exp2( - 16.0 * roughness - 1.0 ) ) - 1.0 + ambientOcclusion );
}`,MM=`
vec3 geometryPosition = - vViewPosition;
vec3 geometryNormal = normal;
vec3 geometryViewDir = ( isOrthographic ) ? vec3( 0, 0, 1 ) : normalize( vViewPosition );
vec3 geometryClearcoatNormal = vec3( 0.0 );
#ifdef USE_CLEARCOAT
	geometryClearcoatNormal = clearcoatNormal;
#endif
#ifdef USE_IRIDESCENCE
	float dotNVi = saturate( dot( normal, geometryViewDir ) );
	if ( material.iridescenceThickness == 0.0 ) {
		material.iridescence = 0.0;
	} else {
		material.iridescence = saturate( material.iridescence );
	}
	if ( material.iridescence > 0.0 ) {
		material.iridescenceFresnelDielectric = evalIridescence( 1.0, material.iridescenceIOR, dotNVi, material.iridescenceThickness, material.specularColor );
		material.iridescenceFresnelMetallic = evalIridescence( 1.0, material.iridescenceIOR, dotNVi, material.iridescenceThickness, material.diffuseColor );
		material.iridescenceFresnel = mix( material.iridescenceFresnelDielectric, material.iridescenceFresnelMetallic, material.metalness );
		material.iridescenceF0 = Schlick_to_F0( material.iridescenceFresnel, 1.0, dotNVi );
	}
#endif
IncidentLight directLight;
#if ( NUM_POINT_LIGHTS > 0 ) && defined( RE_Direct )
	PointLight pointLight;
	#if defined( USE_SHADOWMAP ) && NUM_POINT_LIGHT_SHADOWS > 0
	PointLightShadow pointLightShadow;
	#endif
	#pragma unroll_loop_start
	for ( int i = 0; i < NUM_POINT_LIGHTS; i ++ ) {
		pointLight = pointLights[ i ];
		getPointLightInfo( pointLight, geometryPosition, directLight );
		#if defined( USE_SHADOWMAP ) && ( UNROLLED_LOOP_INDEX < NUM_POINT_LIGHT_SHADOWS ) && ( defined( SHADOWMAP_TYPE_PCF ) || defined( SHADOWMAP_TYPE_BASIC ) )
		pointLightShadow = pointLightShadows[ i ];
		directLight.color *= ( directLight.visible && receiveShadow ) ? getPointShadow( pointShadowMap[ i ], pointLightShadow.shadowMapSize, pointLightShadow.shadowIntensity, pointLightShadow.shadowBias, pointLightShadow.shadowRadius, vPointShadowCoord[ i ], pointLightShadow.shadowCameraNear, pointLightShadow.shadowCameraFar ) : 1.0;
		#endif
		RE_Direct( directLight, geometryPosition, geometryNormal, geometryViewDir, geometryClearcoatNormal, material, reflectedLight );
	}
	#pragma unroll_loop_end
#endif
#if ( NUM_SPOT_LIGHTS > 0 ) && defined( RE_Direct )
	SpotLight spotLight;
	vec4 spotColor;
	vec3 spotLightCoord;
	bool inSpotLightMap;
	#if defined( USE_SHADOWMAP ) && NUM_SPOT_LIGHT_SHADOWS > 0
	SpotLightShadow spotLightShadow;
	#endif
	#pragma unroll_loop_start
	for ( int i = 0; i < NUM_SPOT_LIGHTS; i ++ ) {
		spotLight = spotLights[ i ];
		getSpotLightInfo( spotLight, geometryPosition, directLight );
		#if ( UNROLLED_LOOP_INDEX < NUM_SPOT_LIGHT_SHADOWS_WITH_MAPS )
		#define SPOT_LIGHT_MAP_INDEX UNROLLED_LOOP_INDEX
		#elif ( UNROLLED_LOOP_INDEX < NUM_SPOT_LIGHT_SHADOWS )
		#define SPOT_LIGHT_MAP_INDEX NUM_SPOT_LIGHT_MAPS
		#else
		#define SPOT_LIGHT_MAP_INDEX ( UNROLLED_LOOP_INDEX - NUM_SPOT_LIGHT_SHADOWS + NUM_SPOT_LIGHT_SHADOWS_WITH_MAPS )
		#endif
		#if ( SPOT_LIGHT_MAP_INDEX < NUM_SPOT_LIGHT_MAPS )
			spotLightCoord = vSpotLightCoord[ i ].xyz / vSpotLightCoord[ i ].w;
			inSpotLightMap = all( lessThan( abs( spotLightCoord * 2. - 1. ), vec3( 1.0 ) ) );
			spotColor = texture2D( spotLightMap[ SPOT_LIGHT_MAP_INDEX ], spotLightCoord.xy );
			directLight.color = inSpotLightMap ? directLight.color * spotColor.rgb : directLight.color;
		#endif
		#undef SPOT_LIGHT_MAP_INDEX
		#if defined( USE_SHADOWMAP ) && ( UNROLLED_LOOP_INDEX < NUM_SPOT_LIGHT_SHADOWS )
		spotLightShadow = spotLightShadows[ i ];
		directLight.color *= ( directLight.visible && receiveShadow ) ? getShadow( spotShadowMap[ i ], spotLightShadow.shadowMapSize, spotLightShadow.shadowIntensity, spotLightShadow.shadowBias, spotLightShadow.shadowRadius, vSpotLightCoord[ i ] ) : 1.0;
		#endif
		RE_Direct( directLight, geometryPosition, geometryNormal, geometryViewDir, geometryClearcoatNormal, material, reflectedLight );
	}
	#pragma unroll_loop_end
#endif
#if ( NUM_DIR_LIGHTS > 0 ) && defined( RE_Direct )
	DirectionalLight directionalLight;
	#if defined( USE_SHADOWMAP ) && NUM_DIR_LIGHT_SHADOWS > 0
	DirectionalLightShadow directionalLightShadow;
	#endif
	#pragma unroll_loop_start
	for ( int i = 0; i < NUM_DIR_LIGHTS; i ++ ) {
		directionalLight = directionalLights[ i ];
		getDirectionalLightInfo( directionalLight, directLight );
		#if defined( USE_SHADOWMAP ) && ( UNROLLED_LOOP_INDEX < NUM_DIR_LIGHT_SHADOWS )
		directionalLightShadow = directionalLightShadows[ i ];
		directLight.color *= ( directLight.visible && receiveShadow ) ? getShadow( directionalShadowMap[ i ], directionalLightShadow.shadowMapSize, directionalLightShadow.shadowIntensity, directionalLightShadow.shadowBias, directionalLightShadow.shadowRadius, vDirectionalShadowCoord[ i ] ) : 1.0;
		#endif
		RE_Direct( directLight, geometryPosition, geometryNormal, geometryViewDir, geometryClearcoatNormal, material, reflectedLight );
	}
	#pragma unroll_loop_end
#endif
#if ( NUM_RECT_AREA_LIGHTS > 0 ) && defined( RE_Direct_RectArea )
	RectAreaLight rectAreaLight;
	#pragma unroll_loop_start
	for ( int i = 0; i < NUM_RECT_AREA_LIGHTS; i ++ ) {
		rectAreaLight = rectAreaLights[ i ];
		RE_Direct_RectArea( rectAreaLight, geometryPosition, geometryNormal, geometryViewDir, geometryClearcoatNormal, material, reflectedLight );
	}
	#pragma unroll_loop_end
#endif
#if defined( RE_IndirectDiffuse )
	vec3 iblIrradiance = vec3( 0.0 );
	vec3 irradiance = getAmbientLightIrradiance( ambientLightColor );
	#if defined( USE_LIGHT_PROBES )
		irradiance += getLightProbeIrradiance( lightProbe, geometryNormal );
	#endif
	#if ( NUM_HEMI_LIGHTS > 0 )
		#pragma unroll_loop_start
		for ( int i = 0; i < NUM_HEMI_LIGHTS; i ++ ) {
			irradiance += getHemisphereLightIrradiance( hemisphereLights[ i ], geometryNormal );
		}
		#pragma unroll_loop_end
	#endif
#endif
#if defined( RE_IndirectSpecular )
	vec3 radiance = vec3( 0.0 );
	vec3 clearcoatRadiance = vec3( 0.0 );
#endif`,EM=`#if defined( RE_IndirectDiffuse )
	#ifdef USE_LIGHTMAP
		vec4 lightMapTexel = texture2D( lightMap, vLightMapUv );
		vec3 lightMapIrradiance = lightMapTexel.rgb * lightMapIntensity;
		irradiance += lightMapIrradiance;
	#endif
	#if defined( USE_ENVMAP ) && defined( ENVMAP_TYPE_CUBE_UV )
		#if defined( STANDARD ) || defined( LAMBERT ) || defined( PHONG )
			iblIrradiance += getIBLIrradiance( geometryNormal );
		#endif
	#endif
#endif
#if defined( USE_ENVMAP ) && defined( RE_IndirectSpecular )
	#ifdef USE_ANISOTROPY
		radiance += getIBLAnisotropyRadiance( geometryViewDir, geometryNormal, material.roughness, material.anisotropyB, material.anisotropy );
	#else
		radiance += getIBLRadiance( geometryViewDir, geometryNormal, material.roughness );
	#endif
	#ifdef USE_CLEARCOAT
		clearcoatRadiance += getIBLRadiance( geometryViewDir, geometryClearcoatNormal, material.clearcoatRoughness );
	#endif
#endif`,TM=`#if defined( RE_IndirectDiffuse )
	#if defined( LAMBERT ) || defined( PHONG )
		irradiance += iblIrradiance;
	#endif
	RE_IndirectDiffuse( irradiance, geometryPosition, geometryNormal, geometryViewDir, geometryClearcoatNormal, material, reflectedLight );
#endif
#if defined( RE_IndirectSpecular )
	RE_IndirectSpecular( radiance, iblIrradiance, clearcoatRadiance, geometryPosition, geometryNormal, geometryViewDir, geometryClearcoatNormal, material, reflectedLight );
#endif`,AM=`#if defined( USE_LOGARITHMIC_DEPTH_BUFFER )
	gl_FragDepth = vIsPerspective == 0.0 ? gl_FragCoord.z : log2( vFragDepth ) * logDepthBufFC * 0.5;
#endif`,RM=`#if defined( USE_LOGARITHMIC_DEPTH_BUFFER )
	uniform float logDepthBufFC;
	varying float vFragDepth;
	varying float vIsPerspective;
#endif`,wM=`#ifdef USE_LOGARITHMIC_DEPTH_BUFFER
	varying float vFragDepth;
	varying float vIsPerspective;
#endif`,CM=`#ifdef USE_LOGARITHMIC_DEPTH_BUFFER
	vFragDepth = 1.0 + gl_Position.w;
	vIsPerspective = float( isPerspectiveMatrix( projectionMatrix ) );
#endif`,DM=`#ifdef USE_MAP
	vec4 sampledDiffuseColor = texture2D( map, vMapUv );
	#ifdef DECODE_VIDEO_TEXTURE
		sampledDiffuseColor = sRGBTransferEOTF( sampledDiffuseColor );
	#endif
	diffuseColor *= sampledDiffuseColor;
#endif`,NM=`#ifdef USE_MAP
	uniform sampler2D map;
#endif`,UM=`#if defined( USE_MAP ) || defined( USE_ALPHAMAP )
	#if defined( USE_POINTS_UV )
		vec2 uv = vUv;
	#else
		vec2 uv = ( uvTransform * vec3( gl_PointCoord.x, 1.0 - gl_PointCoord.y, 1 ) ).xy;
	#endif
#endif
#ifdef USE_MAP
	diffuseColor *= texture2D( map, uv );
#endif
#ifdef USE_ALPHAMAP
	diffuseColor.a *= texture2D( alphaMap, uv ).g;
#endif`,LM=`#if defined( USE_POINTS_UV )
	varying vec2 vUv;
#else
	#if defined( USE_MAP ) || defined( USE_ALPHAMAP )
		uniform mat3 uvTransform;
	#endif
#endif
#ifdef USE_MAP
	uniform sampler2D map;
#endif
#ifdef USE_ALPHAMAP
	uniform sampler2D alphaMap;
#endif`,OM=`float metalnessFactor = metalness;
#ifdef USE_METALNESSMAP
	vec4 texelMetalness = texture2D( metalnessMap, vMetalnessMapUv );
	metalnessFactor *= texelMetalness.b;
#endif`,PM=`#ifdef USE_METALNESSMAP
	uniform sampler2D metalnessMap;
#endif`,IM=`#ifdef USE_INSTANCING_MORPH
	float morphTargetInfluences[ MORPHTARGETS_COUNT ];
	float morphTargetBaseInfluence = texelFetch( morphTexture, ivec2( 0, gl_InstanceID ), 0 ).r;
	for ( int i = 0; i < MORPHTARGETS_COUNT; i ++ ) {
		morphTargetInfluences[i] =  texelFetch( morphTexture, ivec2( i + 1, gl_InstanceID ), 0 ).r;
	}
#endif`,FM=`#if defined( USE_MORPHCOLORS )
	vColor *= morphTargetBaseInfluence;
	for ( int i = 0; i < MORPHTARGETS_COUNT; i ++ ) {
		#if defined( USE_COLOR_ALPHA )
			if ( morphTargetInfluences[ i ] != 0.0 ) vColor += getMorph( gl_VertexID, i, 2 ) * morphTargetInfluences[ i ];
		#elif defined( USE_COLOR )
			if ( morphTargetInfluences[ i ] != 0.0 ) vColor += getMorph( gl_VertexID, i, 2 ).rgb * morphTargetInfluences[ i ];
		#endif
	}
#endif`,zM=`#ifdef USE_MORPHNORMALS
	objectNormal *= morphTargetBaseInfluence;
	for ( int i = 0; i < MORPHTARGETS_COUNT; i ++ ) {
		if ( morphTargetInfluences[ i ] != 0.0 ) objectNormal += getMorph( gl_VertexID, i, 1 ).xyz * morphTargetInfluences[ i ];
	}
#endif`,BM=`#ifdef USE_MORPHTARGETS
	#ifndef USE_INSTANCING_MORPH
		uniform float morphTargetBaseInfluence;
		uniform float morphTargetInfluences[ MORPHTARGETS_COUNT ];
	#endif
	uniform sampler2DArray morphTargetsTexture;
	uniform ivec2 morphTargetsTextureSize;
	vec4 getMorph( const in int vertexIndex, const in int morphTargetIndex, const in int offset ) {
		int texelIndex = vertexIndex * MORPHTARGETS_TEXTURE_STRIDE + offset;
		int y = texelIndex / morphTargetsTextureSize.x;
		int x = texelIndex - y * morphTargetsTextureSize.x;
		ivec3 morphUV = ivec3( x, y, morphTargetIndex );
		return texelFetch( morphTargetsTexture, morphUV, 0 );
	}
#endif`,HM=`#ifdef USE_MORPHTARGETS
	transformed *= morphTargetBaseInfluence;
	for ( int i = 0; i < MORPHTARGETS_COUNT; i ++ ) {
		if ( morphTargetInfluences[ i ] != 0.0 ) transformed += getMorph( gl_VertexID, i, 0 ).xyz * morphTargetInfluences[ i ];
	}
#endif`,GM=`float faceDirection = gl_FrontFacing ? 1.0 : - 1.0;
#ifdef FLAT_SHADED
	vec3 fdx = dFdx( vViewPosition );
	vec3 fdy = dFdy( vViewPosition );
	vec3 normal = normalize( cross( fdx, fdy ) );
#else
	vec3 normal = normalize( vNormal );
	#ifdef DOUBLE_SIDED
		normal *= faceDirection;
	#endif
#endif
#if defined( USE_NORMALMAP_TANGENTSPACE ) || defined( USE_CLEARCOAT_NORMALMAP ) || defined( USE_ANISOTROPY )
	#ifdef USE_TANGENT
		mat3 tbn = mat3( normalize( vTangent ), normalize( vBitangent ), normal );
	#else
		mat3 tbn = getTangentFrame( - vViewPosition, normal,
		#if defined( USE_NORMALMAP )
			vNormalMapUv
		#elif defined( USE_CLEARCOAT_NORMALMAP )
			vClearcoatNormalMapUv
		#else
			vUv
		#endif
		);
	#endif
	#if defined( DOUBLE_SIDED ) && ! defined( FLAT_SHADED )
		tbn[0] *= faceDirection;
		tbn[1] *= faceDirection;
	#endif
#endif
#ifdef USE_CLEARCOAT_NORMALMAP
	#ifdef USE_TANGENT
		mat3 tbn2 = mat3( normalize( vTangent ), normalize( vBitangent ), normal );
	#else
		mat3 tbn2 = getTangentFrame( - vViewPosition, normal, vClearcoatNormalMapUv );
	#endif
	#if defined( DOUBLE_SIDED ) && ! defined( FLAT_SHADED )
		tbn2[0] *= faceDirection;
		tbn2[1] *= faceDirection;
	#endif
#endif
vec3 nonPerturbedNormal = normal;`,VM=`#ifdef USE_NORMALMAP_OBJECTSPACE
	normal = texture2D( normalMap, vNormalMapUv ).xyz * 2.0 - 1.0;
	#ifdef FLIP_SIDED
		normal = - normal;
	#endif
	#ifdef DOUBLE_SIDED
		normal = normal * faceDirection;
	#endif
	normal = normalize( normalMatrix * normal );
#elif defined( USE_NORMALMAP_TANGENTSPACE )
	vec3 mapN = texture2D( normalMap, vNormalMapUv ).xyz * 2.0 - 1.0;
	mapN.xy *= normalScale;
	normal = normalize( tbn * mapN );
#elif defined( USE_BUMPMAP )
	normal = perturbNormalArb( - vViewPosition, normal, dHdxy_fwd(), faceDirection );
#endif`,kM=`#ifndef FLAT_SHADED
	varying vec3 vNormal;
	#ifdef USE_TANGENT
		varying vec3 vTangent;
		varying vec3 vBitangent;
	#endif
#endif`,XM=`#ifndef FLAT_SHADED
	varying vec3 vNormal;
	#ifdef USE_TANGENT
		varying vec3 vTangent;
		varying vec3 vBitangent;
	#endif
#endif`,jM=`#ifndef FLAT_SHADED
	vNormal = normalize( transformedNormal );
	#ifdef USE_TANGENT
		vTangent = normalize( transformedTangent );
		vBitangent = normalize( cross( vNormal, vTangent ) * tangent.w );
	#endif
#endif`,WM=`#ifdef USE_NORMALMAP
	uniform sampler2D normalMap;
	uniform vec2 normalScale;
#endif
#ifdef USE_NORMALMAP_OBJECTSPACE
	uniform mat3 normalMatrix;
#endif
#if ! defined ( USE_TANGENT ) && ( defined ( USE_NORMALMAP_TANGENTSPACE ) || defined ( USE_CLEARCOAT_NORMALMAP ) || defined( USE_ANISOTROPY ) )
	mat3 getTangentFrame( vec3 eye_pos, vec3 surf_norm, vec2 uv ) {
		vec3 q0 = dFdx( eye_pos.xyz );
		vec3 q1 = dFdy( eye_pos.xyz );
		vec2 st0 = dFdx( uv.st );
		vec2 st1 = dFdy( uv.st );
		vec3 N = surf_norm;
		vec3 q1perp = cross( q1, N );
		vec3 q0perp = cross( N, q0 );
		vec3 T = q1perp * st0.x + q0perp * st1.x;
		vec3 B = q1perp * st0.y + q0perp * st1.y;
		float det = max( dot( T, T ), dot( B, B ) );
		float scale = ( det == 0.0 ) ? 0.0 : inversesqrt( det );
		return mat3( T * scale, B * scale, N );
	}
#endif`,qM=`#ifdef USE_CLEARCOAT
	vec3 clearcoatNormal = nonPerturbedNormal;
#endif`,YM=`#ifdef USE_CLEARCOAT_NORMALMAP
	vec3 clearcoatMapN = texture2D( clearcoatNormalMap, vClearcoatNormalMapUv ).xyz * 2.0 - 1.0;
	clearcoatMapN.xy *= clearcoatNormalScale;
	clearcoatNormal = normalize( tbn2 * clearcoatMapN );
#endif`,ZM=`#ifdef USE_CLEARCOATMAP
	uniform sampler2D clearcoatMap;
#endif
#ifdef USE_CLEARCOAT_NORMALMAP
	uniform sampler2D clearcoatNormalMap;
	uniform vec2 clearcoatNormalScale;
#endif
#ifdef USE_CLEARCOAT_ROUGHNESSMAP
	uniform sampler2D clearcoatRoughnessMap;
#endif`,KM=`#ifdef USE_IRIDESCENCEMAP
	uniform sampler2D iridescenceMap;
#endif
#ifdef USE_IRIDESCENCE_THICKNESSMAP
	uniform sampler2D iridescenceThicknessMap;
#endif`,QM=`#ifdef OPAQUE
diffuseColor.a = 1.0;
#endif
#ifdef USE_TRANSMISSION
diffuseColor.a *= material.transmissionAlpha;
#endif
gl_FragColor = vec4( outgoingLight, diffuseColor.a );`,JM=`vec3 packNormalToRGB( const in vec3 normal ) {
	return normalize( normal ) * 0.5 + 0.5;
}
vec3 unpackRGBToNormal( const in vec3 rgb ) {
	return 2.0 * rgb.xyz - 1.0;
}
const float PackUpscale = 256. / 255.;const float UnpackDownscale = 255. / 256.;const float ShiftRight8 = 1. / 256.;
const float Inv255 = 1. / 255.;
const vec4 PackFactors = vec4( 1.0, 256.0, 256.0 * 256.0, 256.0 * 256.0 * 256.0 );
const vec2 UnpackFactors2 = vec2( UnpackDownscale, 1.0 / PackFactors.g );
const vec3 UnpackFactors3 = vec3( UnpackDownscale / PackFactors.rg, 1.0 / PackFactors.b );
const vec4 UnpackFactors4 = vec4( UnpackDownscale / PackFactors.rgb, 1.0 / PackFactors.a );
vec4 packDepthToRGBA( const in float v ) {
	if( v <= 0.0 )
		return vec4( 0., 0., 0., 0. );
	if( v >= 1.0 )
		return vec4( 1., 1., 1., 1. );
	float vuf;
	float af = modf( v * PackFactors.a, vuf );
	float bf = modf( vuf * ShiftRight8, vuf );
	float gf = modf( vuf * ShiftRight8, vuf );
	return vec4( vuf * Inv255, gf * PackUpscale, bf * PackUpscale, af );
}
vec3 packDepthToRGB( const in float v ) {
	if( v <= 0.0 )
		return vec3( 0., 0., 0. );
	if( v >= 1.0 )
		return vec3( 1., 1., 1. );
	float vuf;
	float bf = modf( v * PackFactors.b, vuf );
	float gf = modf( vuf * ShiftRight8, vuf );
	return vec3( vuf * Inv255, gf * PackUpscale, bf );
}
vec2 packDepthToRG( const in float v ) {
	if( v <= 0.0 )
		return vec2( 0., 0. );
	if( v >= 1.0 )
		return vec2( 1., 1. );
	float vuf;
	float gf = modf( v * 256., vuf );
	return vec2( vuf * Inv255, gf );
}
float unpackRGBAToDepth( const in vec4 v ) {
	return dot( v, UnpackFactors4 );
}
float unpackRGBToDepth( const in vec3 v ) {
	return dot( v, UnpackFactors3 );
}
float unpackRGToDepth( const in vec2 v ) {
	return v.r * UnpackFactors2.r + v.g * UnpackFactors2.g;
}
vec4 pack2HalfToRGBA( const in vec2 v ) {
	vec4 r = vec4( v.x, fract( v.x * 255.0 ), v.y, fract( v.y * 255.0 ) );
	return vec4( r.x - r.y / 255.0, r.y, r.z - r.w / 255.0, r.w );
}
vec2 unpackRGBATo2Half( const in vec4 v ) {
	return vec2( v.x + ( v.y / 255.0 ), v.z + ( v.w / 255.0 ) );
}
float viewZToOrthographicDepth( const in float viewZ, const in float near, const in float far ) {
	return ( viewZ + near ) / ( near - far );
}
float orthographicDepthToViewZ( const in float depth, const in float near, const in float far ) {
	#ifdef USE_REVERSED_DEPTH_BUFFER
	
		return depth * ( far - near ) - far;
	#else
		return depth * ( near - far ) - near;
	#endif
}
float viewZToPerspectiveDepth( const in float viewZ, const in float near, const in float far ) {
	return ( ( near + viewZ ) * far ) / ( ( far - near ) * viewZ );
}
float perspectiveDepthToViewZ( const in float depth, const in float near, const in float far ) {
	
	#ifdef USE_REVERSED_DEPTH_BUFFER
		return ( near * far ) / ( ( near - far ) * depth - near );
	#else
		return ( near * far ) / ( ( far - near ) * depth - far );
	#endif
}`,$M=`#ifdef PREMULTIPLIED_ALPHA
	gl_FragColor.rgb *= gl_FragColor.a;
#endif`,eE=`vec4 mvPosition = vec4( transformed, 1.0 );
#ifdef USE_BATCHING
	mvPosition = batchingMatrix * mvPosition;
#endif
#ifdef USE_INSTANCING
	mvPosition = instanceMatrix * mvPosition;
#endif
mvPosition = modelViewMatrix * mvPosition;
gl_Position = projectionMatrix * mvPosition;`,tE=`#ifdef DITHERING
	gl_FragColor.rgb = dithering( gl_FragColor.rgb );
#endif`,nE=`#ifdef DITHERING
	vec3 dithering( vec3 color ) {
		float grid_position = rand( gl_FragCoord.xy );
		vec3 dither_shift_RGB = vec3( 0.25 / 255.0, -0.25 / 255.0, 0.25 / 255.0 );
		dither_shift_RGB = mix( 2.0 * dither_shift_RGB, -2.0 * dither_shift_RGB, grid_position );
		return color + dither_shift_RGB;
	}
#endif`,iE=`float roughnessFactor = roughness;
#ifdef USE_ROUGHNESSMAP
	vec4 texelRoughness = texture2D( roughnessMap, vRoughnessMapUv );
	roughnessFactor *= texelRoughness.g;
#endif`,aE=`#ifdef USE_ROUGHNESSMAP
	uniform sampler2D roughnessMap;
#endif`,sE=`#if NUM_SPOT_LIGHT_COORDS > 0
	varying vec4 vSpotLightCoord[ NUM_SPOT_LIGHT_COORDS ];
#endif
#if NUM_SPOT_LIGHT_MAPS > 0
	uniform sampler2D spotLightMap[ NUM_SPOT_LIGHT_MAPS ];
#endif
#ifdef USE_SHADOWMAP
	#if NUM_DIR_LIGHT_SHADOWS > 0
		#if defined( SHADOWMAP_TYPE_PCF )
			uniform sampler2DShadow directionalShadowMap[ NUM_DIR_LIGHT_SHADOWS ];
		#else
			uniform sampler2D directionalShadowMap[ NUM_DIR_LIGHT_SHADOWS ];
		#endif
		varying vec4 vDirectionalShadowCoord[ NUM_DIR_LIGHT_SHADOWS ];
		struct DirectionalLightShadow {
			float shadowIntensity;
			float shadowBias;
			float shadowNormalBias;
			float shadowRadius;
			vec2 shadowMapSize;
		};
		uniform DirectionalLightShadow directionalLightShadows[ NUM_DIR_LIGHT_SHADOWS ];
	#endif
	#if NUM_SPOT_LIGHT_SHADOWS > 0
		#if defined( SHADOWMAP_TYPE_PCF )
			uniform sampler2DShadow spotShadowMap[ NUM_SPOT_LIGHT_SHADOWS ];
		#else
			uniform sampler2D spotShadowMap[ NUM_SPOT_LIGHT_SHADOWS ];
		#endif
		struct SpotLightShadow {
			float shadowIntensity;
			float shadowBias;
			float shadowNormalBias;
			float shadowRadius;
			vec2 shadowMapSize;
		};
		uniform SpotLightShadow spotLightShadows[ NUM_SPOT_LIGHT_SHADOWS ];
	#endif
	#if NUM_POINT_LIGHT_SHADOWS > 0
		#if defined( SHADOWMAP_TYPE_PCF )
			uniform samplerCubeShadow pointShadowMap[ NUM_POINT_LIGHT_SHADOWS ];
		#elif defined( SHADOWMAP_TYPE_BASIC )
			uniform samplerCube pointShadowMap[ NUM_POINT_LIGHT_SHADOWS ];
		#endif
		varying vec4 vPointShadowCoord[ NUM_POINT_LIGHT_SHADOWS ];
		struct PointLightShadow {
			float shadowIntensity;
			float shadowBias;
			float shadowNormalBias;
			float shadowRadius;
			vec2 shadowMapSize;
			float shadowCameraNear;
			float shadowCameraFar;
		};
		uniform PointLightShadow pointLightShadows[ NUM_POINT_LIGHT_SHADOWS ];
	#endif
	#if defined( SHADOWMAP_TYPE_PCF )
		float interleavedGradientNoise( vec2 position ) {
			return fract( 52.9829189 * fract( dot( position, vec2( 0.06711056, 0.00583715 ) ) ) );
		}
		vec2 vogelDiskSample( int sampleIndex, int samplesCount, float phi ) {
			const float goldenAngle = 2.399963229728653;
			float r = sqrt( ( float( sampleIndex ) + 0.5 ) / float( samplesCount ) );
			float theta = float( sampleIndex ) * goldenAngle + phi;
			return vec2( cos( theta ), sin( theta ) ) * r;
		}
	#endif
	#if defined( SHADOWMAP_TYPE_PCF )
		float getShadow( sampler2DShadow shadowMap, vec2 shadowMapSize, float shadowIntensity, float shadowBias, float shadowRadius, vec4 shadowCoord ) {
			float shadow = 1.0;
			shadowCoord.xyz /= shadowCoord.w;
			shadowCoord.z += shadowBias;
			bool inFrustum = shadowCoord.x >= 0.0 && shadowCoord.x <= 1.0 && shadowCoord.y >= 0.0 && shadowCoord.y <= 1.0;
			bool frustumTest = inFrustum && shadowCoord.z <= 1.0;
			if ( frustumTest ) {
				vec2 texelSize = vec2( 1.0 ) / shadowMapSize;
				float radius = shadowRadius * texelSize.x;
				float phi = interleavedGradientNoise( gl_FragCoord.xy ) * PI2;
				shadow = (
					texture( shadowMap, vec3( shadowCoord.xy + vogelDiskSample( 0, 5, phi ) * radius, shadowCoord.z ) ) +
					texture( shadowMap, vec3( shadowCoord.xy + vogelDiskSample( 1, 5, phi ) * radius, shadowCoord.z ) ) +
					texture( shadowMap, vec3( shadowCoord.xy + vogelDiskSample( 2, 5, phi ) * radius, shadowCoord.z ) ) +
					texture( shadowMap, vec3( shadowCoord.xy + vogelDiskSample( 3, 5, phi ) * radius, shadowCoord.z ) ) +
					texture( shadowMap, vec3( shadowCoord.xy + vogelDiskSample( 4, 5, phi ) * radius, shadowCoord.z ) )
				) * 0.2;
			}
			return mix( 1.0, shadow, shadowIntensity );
		}
	#elif defined( SHADOWMAP_TYPE_VSM )
		float getShadow( sampler2D shadowMap, vec2 shadowMapSize, float shadowIntensity, float shadowBias, float shadowRadius, vec4 shadowCoord ) {
			float shadow = 1.0;
			shadowCoord.xyz /= shadowCoord.w;
			#ifdef USE_REVERSED_DEPTH_BUFFER
				shadowCoord.z -= shadowBias;
			#else
				shadowCoord.z += shadowBias;
			#endif
			bool inFrustum = shadowCoord.x >= 0.0 && shadowCoord.x <= 1.0 && shadowCoord.y >= 0.0 && shadowCoord.y <= 1.0;
			bool frustumTest = inFrustum && shadowCoord.z <= 1.0;
			if ( frustumTest ) {
				vec2 distribution = texture2D( shadowMap, shadowCoord.xy ).rg;
				float mean = distribution.x;
				float variance = distribution.y * distribution.y;
				#ifdef USE_REVERSED_DEPTH_BUFFER
					float hard_shadow = step( mean, shadowCoord.z );
				#else
					float hard_shadow = step( shadowCoord.z, mean );
				#endif
				
				if ( hard_shadow == 1.0 ) {
					shadow = 1.0;
				} else {
					variance = max( variance, 0.0000001 );
					float d = shadowCoord.z - mean;
					float p_max = variance / ( variance + d * d );
					p_max = clamp( ( p_max - 0.3 ) / 0.65, 0.0, 1.0 );
					shadow = max( hard_shadow, p_max );
				}
			}
			return mix( 1.0, shadow, shadowIntensity );
		}
	#else
		float getShadow( sampler2D shadowMap, vec2 shadowMapSize, float shadowIntensity, float shadowBias, float shadowRadius, vec4 shadowCoord ) {
			float shadow = 1.0;
			shadowCoord.xyz /= shadowCoord.w;
			#ifdef USE_REVERSED_DEPTH_BUFFER
				shadowCoord.z -= shadowBias;
			#else
				shadowCoord.z += shadowBias;
			#endif
			bool inFrustum = shadowCoord.x >= 0.0 && shadowCoord.x <= 1.0 && shadowCoord.y >= 0.0 && shadowCoord.y <= 1.0;
			bool frustumTest = inFrustum && shadowCoord.z <= 1.0;
			if ( frustumTest ) {
				float depth = texture2D( shadowMap, shadowCoord.xy ).r;
				#ifdef USE_REVERSED_DEPTH_BUFFER
					shadow = step( depth, shadowCoord.z );
				#else
					shadow = step( shadowCoord.z, depth );
				#endif
			}
			return mix( 1.0, shadow, shadowIntensity );
		}
	#endif
	#if NUM_POINT_LIGHT_SHADOWS > 0
	#if defined( SHADOWMAP_TYPE_PCF )
	float getPointShadow( samplerCubeShadow shadowMap, vec2 shadowMapSize, float shadowIntensity, float shadowBias, float shadowRadius, vec4 shadowCoord, float shadowCameraNear, float shadowCameraFar ) {
		float shadow = 1.0;
		vec3 lightToPosition = shadowCoord.xyz;
		vec3 bd3D = normalize( lightToPosition );
		vec3 absVec = abs( lightToPosition );
		float viewSpaceZ = max( max( absVec.x, absVec.y ), absVec.z );
		if ( viewSpaceZ - shadowCameraFar <= 0.0 && viewSpaceZ - shadowCameraNear >= 0.0 ) {
			#ifdef USE_REVERSED_DEPTH_BUFFER
				float dp = ( shadowCameraNear * ( shadowCameraFar - viewSpaceZ ) ) / ( viewSpaceZ * ( shadowCameraFar - shadowCameraNear ) );
				dp -= shadowBias;
			#else
				float dp = ( shadowCameraFar * ( viewSpaceZ - shadowCameraNear ) ) / ( viewSpaceZ * ( shadowCameraFar - shadowCameraNear ) );
				dp += shadowBias;
			#endif
			float texelSize = shadowRadius / shadowMapSize.x;
			vec3 absDir = abs( bd3D );
			vec3 tangent = absDir.x > absDir.z ? vec3( 0.0, 1.0, 0.0 ) : vec3( 1.0, 0.0, 0.0 );
			tangent = normalize( cross( bd3D, tangent ) );
			vec3 bitangent = cross( bd3D, tangent );
			float phi = interleavedGradientNoise( gl_FragCoord.xy ) * PI2;
			vec2 sample0 = vogelDiskSample( 0, 5, phi );
			vec2 sample1 = vogelDiskSample( 1, 5, phi );
			vec2 sample2 = vogelDiskSample( 2, 5, phi );
			vec2 sample3 = vogelDiskSample( 3, 5, phi );
			vec2 sample4 = vogelDiskSample( 4, 5, phi );
			shadow = (
				texture( shadowMap, vec4( bd3D + ( tangent * sample0.x + bitangent * sample0.y ) * texelSize, dp ) ) +
				texture( shadowMap, vec4( bd3D + ( tangent * sample1.x + bitangent * sample1.y ) * texelSize, dp ) ) +
				texture( shadowMap, vec4( bd3D + ( tangent * sample2.x + bitangent * sample2.y ) * texelSize, dp ) ) +
				texture( shadowMap, vec4( bd3D + ( tangent * sample3.x + bitangent * sample3.y ) * texelSize, dp ) ) +
				texture( shadowMap, vec4( bd3D + ( tangent * sample4.x + bitangent * sample4.y ) * texelSize, dp ) )
			) * 0.2;
		}
		return mix( 1.0, shadow, shadowIntensity );
	}
	#elif defined( SHADOWMAP_TYPE_BASIC )
	float getPointShadow( samplerCube shadowMap, vec2 shadowMapSize, float shadowIntensity, float shadowBias, float shadowRadius, vec4 shadowCoord, float shadowCameraNear, float shadowCameraFar ) {
		float shadow = 1.0;
		vec3 lightToPosition = shadowCoord.xyz;
		vec3 absVec = abs( lightToPosition );
		float viewSpaceZ = max( max( absVec.x, absVec.y ), absVec.z );
		if ( viewSpaceZ - shadowCameraFar <= 0.0 && viewSpaceZ - shadowCameraNear >= 0.0 ) {
			float dp = ( shadowCameraFar * ( viewSpaceZ - shadowCameraNear ) ) / ( viewSpaceZ * ( shadowCameraFar - shadowCameraNear ) );
			dp += shadowBias;
			vec3 bd3D = normalize( lightToPosition );
			float depth = textureCube( shadowMap, bd3D ).r;
			#ifdef USE_REVERSED_DEPTH_BUFFER
				depth = 1.0 - depth;
			#endif
			shadow = step( dp, depth );
		}
		return mix( 1.0, shadow, shadowIntensity );
	}
	#endif
	#endif
#endif`,rE=`#if NUM_SPOT_LIGHT_COORDS > 0
	uniform mat4 spotLightMatrix[ NUM_SPOT_LIGHT_COORDS ];
	varying vec4 vSpotLightCoord[ NUM_SPOT_LIGHT_COORDS ];
#endif
#ifdef USE_SHADOWMAP
	#if NUM_DIR_LIGHT_SHADOWS > 0
		uniform mat4 directionalShadowMatrix[ NUM_DIR_LIGHT_SHADOWS ];
		varying vec4 vDirectionalShadowCoord[ NUM_DIR_LIGHT_SHADOWS ];
		struct DirectionalLightShadow {
			float shadowIntensity;
			float shadowBias;
			float shadowNormalBias;
			float shadowRadius;
			vec2 shadowMapSize;
		};
		uniform DirectionalLightShadow directionalLightShadows[ NUM_DIR_LIGHT_SHADOWS ];
	#endif
	#if NUM_SPOT_LIGHT_SHADOWS > 0
		struct SpotLightShadow {
			float shadowIntensity;
			float shadowBias;
			float shadowNormalBias;
			float shadowRadius;
			vec2 shadowMapSize;
		};
		uniform SpotLightShadow spotLightShadows[ NUM_SPOT_LIGHT_SHADOWS ];
	#endif
	#if NUM_POINT_LIGHT_SHADOWS > 0
		uniform mat4 pointShadowMatrix[ NUM_POINT_LIGHT_SHADOWS ];
		varying vec4 vPointShadowCoord[ NUM_POINT_LIGHT_SHADOWS ];
		struct PointLightShadow {
			float shadowIntensity;
			float shadowBias;
			float shadowNormalBias;
			float shadowRadius;
			vec2 shadowMapSize;
			float shadowCameraNear;
			float shadowCameraFar;
		};
		uniform PointLightShadow pointLightShadows[ NUM_POINT_LIGHT_SHADOWS ];
	#endif
#endif`,oE=`#if ( defined( USE_SHADOWMAP ) && ( NUM_DIR_LIGHT_SHADOWS > 0 || NUM_POINT_LIGHT_SHADOWS > 0 ) ) || ( NUM_SPOT_LIGHT_COORDS > 0 )
	vec3 shadowWorldNormal = inverseTransformDirection( transformedNormal, viewMatrix );
	vec4 shadowWorldPosition;
#endif
#if defined( USE_SHADOWMAP )
	#if NUM_DIR_LIGHT_SHADOWS > 0
		#pragma unroll_loop_start
		for ( int i = 0; i < NUM_DIR_LIGHT_SHADOWS; i ++ ) {
			shadowWorldPosition = worldPosition + vec4( shadowWorldNormal * directionalLightShadows[ i ].shadowNormalBias, 0 );
			vDirectionalShadowCoord[ i ] = directionalShadowMatrix[ i ] * shadowWorldPosition;
		}
		#pragma unroll_loop_end
	#endif
	#if NUM_POINT_LIGHT_SHADOWS > 0
		#pragma unroll_loop_start
		for ( int i = 0; i < NUM_POINT_LIGHT_SHADOWS; i ++ ) {
			shadowWorldPosition = worldPosition + vec4( shadowWorldNormal * pointLightShadows[ i ].shadowNormalBias, 0 );
			vPointShadowCoord[ i ] = pointShadowMatrix[ i ] * shadowWorldPosition;
		}
		#pragma unroll_loop_end
	#endif
#endif
#if NUM_SPOT_LIGHT_COORDS > 0
	#pragma unroll_loop_start
	for ( int i = 0; i < NUM_SPOT_LIGHT_COORDS; i ++ ) {
		shadowWorldPosition = worldPosition;
		#if ( defined( USE_SHADOWMAP ) && UNROLLED_LOOP_INDEX < NUM_SPOT_LIGHT_SHADOWS )
			shadowWorldPosition.xyz += shadowWorldNormal * spotLightShadows[ i ].shadowNormalBias;
		#endif
		vSpotLightCoord[ i ] = spotLightMatrix[ i ] * shadowWorldPosition;
	}
	#pragma unroll_loop_end
#endif`,lE=`float getShadowMask() {
	float shadow = 1.0;
	#ifdef USE_SHADOWMAP
	#if NUM_DIR_LIGHT_SHADOWS > 0
	DirectionalLightShadow directionalLight;
	#pragma unroll_loop_start
	for ( int i = 0; i < NUM_DIR_LIGHT_SHADOWS; i ++ ) {
		directionalLight = directionalLightShadows[ i ];
		shadow *= receiveShadow ? getShadow( directionalShadowMap[ i ], directionalLight.shadowMapSize, directionalLight.shadowIntensity, directionalLight.shadowBias, directionalLight.shadowRadius, vDirectionalShadowCoord[ i ] ) : 1.0;
	}
	#pragma unroll_loop_end
	#endif
	#if NUM_SPOT_LIGHT_SHADOWS > 0
	SpotLightShadow spotLight;
	#pragma unroll_loop_start
	for ( int i = 0; i < NUM_SPOT_LIGHT_SHADOWS; i ++ ) {
		spotLight = spotLightShadows[ i ];
		shadow *= receiveShadow ? getShadow( spotShadowMap[ i ], spotLight.shadowMapSize, spotLight.shadowIntensity, spotLight.shadowBias, spotLight.shadowRadius, vSpotLightCoord[ i ] ) : 1.0;
	}
	#pragma unroll_loop_end
	#endif
	#if NUM_POINT_LIGHT_SHADOWS > 0 && ( defined( SHADOWMAP_TYPE_PCF ) || defined( SHADOWMAP_TYPE_BASIC ) )
	PointLightShadow pointLight;
	#pragma unroll_loop_start
	for ( int i = 0; i < NUM_POINT_LIGHT_SHADOWS; i ++ ) {
		pointLight = pointLightShadows[ i ];
		shadow *= receiveShadow ? getPointShadow( pointShadowMap[ i ], pointLight.shadowMapSize, pointLight.shadowIntensity, pointLight.shadowBias, pointLight.shadowRadius, vPointShadowCoord[ i ], pointLight.shadowCameraNear, pointLight.shadowCameraFar ) : 1.0;
	}
	#pragma unroll_loop_end
	#endif
	#endif
	return shadow;
}`,cE=`#ifdef USE_SKINNING
	mat4 boneMatX = getBoneMatrix( skinIndex.x );
	mat4 boneMatY = getBoneMatrix( skinIndex.y );
	mat4 boneMatZ = getBoneMatrix( skinIndex.z );
	mat4 boneMatW = getBoneMatrix( skinIndex.w );
#endif`,uE=`#ifdef USE_SKINNING
	uniform mat4 bindMatrix;
	uniform mat4 bindMatrixInverse;
	uniform highp sampler2D boneTexture;
	mat4 getBoneMatrix( const in float i ) {
		int size = textureSize( boneTexture, 0 ).x;
		int j = int( i ) * 4;
		int x = j % size;
		int y = j / size;
		vec4 v1 = texelFetch( boneTexture, ivec2( x, y ), 0 );
		vec4 v2 = texelFetch( boneTexture, ivec2( x + 1, y ), 0 );
		vec4 v3 = texelFetch( boneTexture, ivec2( x + 2, y ), 0 );
		vec4 v4 = texelFetch( boneTexture, ivec2( x + 3, y ), 0 );
		return mat4( v1, v2, v3, v4 );
	}
#endif`,fE=`#ifdef USE_SKINNING
	vec4 skinVertex = bindMatrix * vec4( transformed, 1.0 );
	vec4 skinned = vec4( 0.0 );
	skinned += boneMatX * skinVertex * skinWeight.x;
	skinned += boneMatY * skinVertex * skinWeight.y;
	skinned += boneMatZ * skinVertex * skinWeight.z;
	skinned += boneMatW * skinVertex * skinWeight.w;
	transformed = ( bindMatrixInverse * skinned ).xyz;
#endif`,dE=`#ifdef USE_SKINNING
	mat4 skinMatrix = mat4( 0.0 );
	skinMatrix += skinWeight.x * boneMatX;
	skinMatrix += skinWeight.y * boneMatY;
	skinMatrix += skinWeight.z * boneMatZ;
	skinMatrix += skinWeight.w * boneMatW;
	skinMatrix = bindMatrixInverse * skinMatrix * bindMatrix;
	objectNormal = vec4( skinMatrix * vec4( objectNormal, 0.0 ) ).xyz;
	#ifdef USE_TANGENT
		objectTangent = vec4( skinMatrix * vec4( objectTangent, 0.0 ) ).xyz;
	#endif
#endif`,hE=`float specularStrength;
#ifdef USE_SPECULARMAP
	vec4 texelSpecular = texture2D( specularMap, vSpecularMapUv );
	specularStrength = texelSpecular.r;
#else
	specularStrength = 1.0;
#endif`,pE=`#ifdef USE_SPECULARMAP
	uniform sampler2D specularMap;
#endif`,mE=`#if defined( TONE_MAPPING )
	gl_FragColor.rgb = toneMapping( gl_FragColor.rgb );
#endif`,gE=`#ifndef saturate
#define saturate( a ) clamp( a, 0.0, 1.0 )
#endif
uniform float toneMappingExposure;
vec3 LinearToneMapping( vec3 color ) {
	return saturate( toneMappingExposure * color );
}
vec3 ReinhardToneMapping( vec3 color ) {
	color *= toneMappingExposure;
	return saturate( color / ( vec3( 1.0 ) + color ) );
}
vec3 CineonToneMapping( vec3 color ) {
	color *= toneMappingExposure;
	color = max( vec3( 0.0 ), color - 0.004 );
	return pow( ( color * ( 6.2 * color + 0.5 ) ) / ( color * ( 6.2 * color + 1.7 ) + 0.06 ), vec3( 2.2 ) );
}
vec3 RRTAndODTFit( vec3 v ) {
	vec3 a = v * ( v + 0.0245786 ) - 0.000090537;
	vec3 b = v * ( 0.983729 * v + 0.4329510 ) + 0.238081;
	return a / b;
}
vec3 ACESFilmicToneMapping( vec3 color ) {
	const mat3 ACESInputMat = mat3(
		vec3( 0.59719, 0.07600, 0.02840 ),		vec3( 0.35458, 0.90834, 0.13383 ),
		vec3( 0.04823, 0.01566, 0.83777 )
	);
	const mat3 ACESOutputMat = mat3(
		vec3(  1.60475, -0.10208, -0.00327 ),		vec3( -0.53108,  1.10813, -0.07276 ),
		vec3( -0.07367, -0.00605,  1.07602 )
	);
	color *= toneMappingExposure / 0.6;
	color = ACESInputMat * color;
	color = RRTAndODTFit( color );
	color = ACESOutputMat * color;
	return saturate( color );
}
const mat3 LINEAR_REC2020_TO_LINEAR_SRGB = mat3(
	vec3( 1.6605, - 0.1246, - 0.0182 ),
	vec3( - 0.5876, 1.1329, - 0.1006 ),
	vec3( - 0.0728, - 0.0083, 1.1187 )
);
const mat3 LINEAR_SRGB_TO_LINEAR_REC2020 = mat3(
	vec3( 0.6274, 0.0691, 0.0164 ),
	vec3( 0.3293, 0.9195, 0.0880 ),
	vec3( 0.0433, 0.0113, 0.8956 )
);
vec3 agxDefaultContrastApprox( vec3 x ) {
	vec3 x2 = x * x;
	vec3 x4 = x2 * x2;
	return + 15.5 * x4 * x2
		- 40.14 * x4 * x
		+ 31.96 * x4
		- 6.868 * x2 * x
		+ 0.4298 * x2
		+ 0.1191 * x
		- 0.00232;
}
vec3 AgXToneMapping( vec3 color ) {
	const mat3 AgXInsetMatrix = mat3(
		vec3( 0.856627153315983, 0.137318972929847, 0.11189821299995 ),
		vec3( 0.0951212405381588, 0.761241990602591, 0.0767994186031903 ),
		vec3( 0.0482516061458583, 0.101439036467562, 0.811302368396859 )
	);
	const mat3 AgXOutsetMatrix = mat3(
		vec3( 1.1271005818144368, - 0.1413297634984383, - 0.14132976349843826 ),
		vec3( - 0.11060664309660323, 1.157823702216272, - 0.11060664309660294 ),
		vec3( - 0.016493938717834573, - 0.016493938717834257, 1.2519364065950405 )
	);
	const float AgxMinEv = - 12.47393;	const float AgxMaxEv = 4.026069;
	color *= toneMappingExposure;
	color = LINEAR_SRGB_TO_LINEAR_REC2020 * color;
	color = AgXInsetMatrix * color;
	color = max( color, 1e-10 );	color = log2( color );
	color = ( color - AgxMinEv ) / ( AgxMaxEv - AgxMinEv );
	color = clamp( color, 0.0, 1.0 );
	color = agxDefaultContrastApprox( color );
	color = AgXOutsetMatrix * color;
	color = pow( max( vec3( 0.0 ), color ), vec3( 2.2 ) );
	color = LINEAR_REC2020_TO_LINEAR_SRGB * color;
	color = clamp( color, 0.0, 1.0 );
	return color;
}
vec3 NeutralToneMapping( vec3 color ) {
	const float StartCompression = 0.8 - 0.04;
	const float Desaturation = 0.15;
	color *= toneMappingExposure;
	float x = min( color.r, min( color.g, color.b ) );
	float offset = x < 0.08 ? x - 6.25 * x * x : 0.04;
	color -= offset;
	float peak = max( color.r, max( color.g, color.b ) );
	if ( peak < StartCompression ) return color;
	float d = 1. - StartCompression;
	float newPeak = 1. - d * d / ( peak + d - StartCompression );
	color *= newPeak / peak;
	float g = 1. - 1. / ( Desaturation * ( peak - newPeak ) + 1. );
	return mix( color, vec3( newPeak ), g );
}
vec3 CustomToneMapping( vec3 color ) { return color; }`,_E=`#ifdef USE_TRANSMISSION
	material.transmission = transmission;
	material.transmissionAlpha = 1.0;
	material.thickness = thickness;
	material.attenuationDistance = attenuationDistance;
	material.attenuationColor = attenuationColor;
	#ifdef USE_TRANSMISSIONMAP
		material.transmission *= texture2D( transmissionMap, vTransmissionMapUv ).r;
	#endif
	#ifdef USE_THICKNESSMAP
		material.thickness *= texture2D( thicknessMap, vThicknessMapUv ).g;
	#endif
	vec3 pos = vWorldPosition;
	vec3 v = normalize( cameraPosition - pos );
	vec3 n = inverseTransformDirection( normal, viewMatrix );
	vec4 transmitted = getIBLVolumeRefraction(
		n, v, material.roughness, material.diffuseContribution, material.specularColorBlended, material.specularF90,
		pos, modelMatrix, viewMatrix, projectionMatrix, material.dispersion, material.ior, material.thickness,
		material.attenuationColor, material.attenuationDistance );
	material.transmissionAlpha = mix( material.transmissionAlpha, transmitted.a, material.transmission );
	totalDiffuse = mix( totalDiffuse, transmitted.rgb, material.transmission );
#endif`,vE=`#ifdef USE_TRANSMISSION
	uniform float transmission;
	uniform float thickness;
	uniform float attenuationDistance;
	uniform vec3 attenuationColor;
	#ifdef USE_TRANSMISSIONMAP
		uniform sampler2D transmissionMap;
	#endif
	#ifdef USE_THICKNESSMAP
		uniform sampler2D thicknessMap;
	#endif
	uniform vec2 transmissionSamplerSize;
	uniform sampler2D transmissionSamplerMap;
	uniform mat4 modelMatrix;
	uniform mat4 projectionMatrix;
	varying vec3 vWorldPosition;
	float w0( float a ) {
		return ( 1.0 / 6.0 ) * ( a * ( a * ( - a + 3.0 ) - 3.0 ) + 1.0 );
	}
	float w1( float a ) {
		return ( 1.0 / 6.0 ) * ( a *  a * ( 3.0 * a - 6.0 ) + 4.0 );
	}
	float w2( float a ){
		return ( 1.0 / 6.0 ) * ( a * ( a * ( - 3.0 * a + 3.0 ) + 3.0 ) + 1.0 );
	}
	float w3( float a ) {
		return ( 1.0 / 6.0 ) * ( a * a * a );
	}
	float g0( float a ) {
		return w0( a ) + w1( a );
	}
	float g1( float a ) {
		return w2( a ) + w3( a );
	}
	float h0( float a ) {
		return - 1.0 + w1( a ) / ( w0( a ) + w1( a ) );
	}
	float h1( float a ) {
		return 1.0 + w3( a ) / ( w2( a ) + w3( a ) );
	}
	vec4 bicubic( sampler2D tex, vec2 uv, vec4 texelSize, float lod ) {
		uv = uv * texelSize.zw + 0.5;
		vec2 iuv = floor( uv );
		vec2 fuv = fract( uv );
		float g0x = g0( fuv.x );
		float g1x = g1( fuv.x );
		float h0x = h0( fuv.x );
		float h1x = h1( fuv.x );
		float h0y = h0( fuv.y );
		float h1y = h1( fuv.y );
		vec2 p0 = ( vec2( iuv.x + h0x, iuv.y + h0y ) - 0.5 ) * texelSize.xy;
		vec2 p1 = ( vec2( iuv.x + h1x, iuv.y + h0y ) - 0.5 ) * texelSize.xy;
		vec2 p2 = ( vec2( iuv.x + h0x, iuv.y + h1y ) - 0.5 ) * texelSize.xy;
		vec2 p3 = ( vec2( iuv.x + h1x, iuv.y + h1y ) - 0.5 ) * texelSize.xy;
		return g0( fuv.y ) * ( g0x * textureLod( tex, p0, lod ) + g1x * textureLod( tex, p1, lod ) ) +
			g1( fuv.y ) * ( g0x * textureLod( tex, p2, lod ) + g1x * textureLod( tex, p3, lod ) );
	}
	vec4 textureBicubic( sampler2D sampler, vec2 uv, float lod ) {
		vec2 fLodSize = vec2( textureSize( sampler, int( lod ) ) );
		vec2 cLodSize = vec2( textureSize( sampler, int( lod + 1.0 ) ) );
		vec2 fLodSizeInv = 1.0 / fLodSize;
		vec2 cLodSizeInv = 1.0 / cLodSize;
		vec4 fSample = bicubic( sampler, uv, vec4( fLodSizeInv, fLodSize ), floor( lod ) );
		vec4 cSample = bicubic( sampler, uv, vec4( cLodSizeInv, cLodSize ), ceil( lod ) );
		return mix( fSample, cSample, fract( lod ) );
	}
	vec3 getVolumeTransmissionRay( const in vec3 n, const in vec3 v, const in float thickness, const in float ior, const in mat4 modelMatrix ) {
		vec3 refractionVector = refract( - v, normalize( n ), 1.0 / ior );
		vec3 modelScale;
		modelScale.x = length( vec3( modelMatrix[ 0 ].xyz ) );
		modelScale.y = length( vec3( modelMatrix[ 1 ].xyz ) );
		modelScale.z = length( vec3( modelMatrix[ 2 ].xyz ) );
		return normalize( refractionVector ) * thickness * modelScale;
	}
	float applyIorToRoughness( const in float roughness, const in float ior ) {
		return roughness * clamp( ior * 2.0 - 2.0, 0.0, 1.0 );
	}
	vec4 getTransmissionSample( const in vec2 fragCoord, const in float roughness, const in float ior ) {
		float lod = log2( transmissionSamplerSize.x ) * applyIorToRoughness( roughness, ior );
		return textureBicubic( transmissionSamplerMap, fragCoord.xy, lod );
	}
	vec3 volumeAttenuation( const in float transmissionDistance, const in vec3 attenuationColor, const in float attenuationDistance ) {
		if ( isinf( attenuationDistance ) ) {
			return vec3( 1.0 );
		} else {
			vec3 attenuationCoefficient = -log( attenuationColor ) / attenuationDistance;
			vec3 transmittance = exp( - attenuationCoefficient * transmissionDistance );			return transmittance;
		}
	}
	vec4 getIBLVolumeRefraction( const in vec3 n, const in vec3 v, const in float roughness, const in vec3 diffuseColor,
		const in vec3 specularColor, const in float specularF90, const in vec3 position, const in mat4 modelMatrix,
		const in mat4 viewMatrix, const in mat4 projMatrix, const in float dispersion, const in float ior, const in float thickness,
		const in vec3 attenuationColor, const in float attenuationDistance ) {
		vec4 transmittedLight;
		vec3 transmittance;
		#ifdef USE_DISPERSION
			float halfSpread = ( ior - 1.0 ) * 0.025 * dispersion;
			vec3 iors = vec3( ior - halfSpread, ior, ior + halfSpread );
			for ( int i = 0; i < 3; i ++ ) {
				vec3 transmissionRay = getVolumeTransmissionRay( n, v, thickness, iors[ i ], modelMatrix );
				vec3 refractedRayExit = position + transmissionRay;
				vec4 ndcPos = projMatrix * viewMatrix * vec4( refractedRayExit, 1.0 );
				vec2 refractionCoords = ndcPos.xy / ndcPos.w;
				refractionCoords += 1.0;
				refractionCoords /= 2.0;
				vec4 transmissionSample = getTransmissionSample( refractionCoords, roughness, iors[ i ] );
				transmittedLight[ i ] = transmissionSample[ i ];
				transmittedLight.a += transmissionSample.a;
				transmittance[ i ] = diffuseColor[ i ] * volumeAttenuation( length( transmissionRay ), attenuationColor, attenuationDistance )[ i ];
			}
			transmittedLight.a /= 3.0;
		#else
			vec3 transmissionRay = getVolumeTransmissionRay( n, v, thickness, ior, modelMatrix );
			vec3 refractedRayExit = position + transmissionRay;
			vec4 ndcPos = projMatrix * viewMatrix * vec4( refractedRayExit, 1.0 );
			vec2 refractionCoords = ndcPos.xy / ndcPos.w;
			refractionCoords += 1.0;
			refractionCoords /= 2.0;
			transmittedLight = getTransmissionSample( refractionCoords, roughness, ior );
			transmittance = diffuseColor * volumeAttenuation( length( transmissionRay ), attenuationColor, attenuationDistance );
		#endif
		vec3 attenuatedColor = transmittance * transmittedLight.rgb;
		vec3 F = EnvironmentBRDF( n, v, specularColor, specularF90, roughness );
		float transmittanceFactor = ( transmittance.r + transmittance.g + transmittance.b ) / 3.0;
		return vec4( ( 1.0 - F ) * attenuatedColor, 1.0 - ( 1.0 - transmittedLight.a ) * transmittanceFactor );
	}
#endif`,xE=`#if defined( USE_UV ) || defined( USE_ANISOTROPY )
	varying vec2 vUv;
#endif
#ifdef USE_MAP
	varying vec2 vMapUv;
#endif
#ifdef USE_ALPHAMAP
	varying vec2 vAlphaMapUv;
#endif
#ifdef USE_LIGHTMAP
	varying vec2 vLightMapUv;
#endif
#ifdef USE_AOMAP
	varying vec2 vAoMapUv;
#endif
#ifdef USE_BUMPMAP
	varying vec2 vBumpMapUv;
#endif
#ifdef USE_NORMALMAP
	varying vec2 vNormalMapUv;
#endif
#ifdef USE_EMISSIVEMAP
	varying vec2 vEmissiveMapUv;
#endif
#ifdef USE_METALNESSMAP
	varying vec2 vMetalnessMapUv;
#endif
#ifdef USE_ROUGHNESSMAP
	varying vec2 vRoughnessMapUv;
#endif
#ifdef USE_ANISOTROPYMAP
	varying vec2 vAnisotropyMapUv;
#endif
#ifdef USE_CLEARCOATMAP
	varying vec2 vClearcoatMapUv;
#endif
#ifdef USE_CLEARCOAT_NORMALMAP
	varying vec2 vClearcoatNormalMapUv;
#endif
#ifdef USE_CLEARCOAT_ROUGHNESSMAP
	varying vec2 vClearcoatRoughnessMapUv;
#endif
#ifdef USE_IRIDESCENCEMAP
	varying vec2 vIridescenceMapUv;
#endif
#ifdef USE_IRIDESCENCE_THICKNESSMAP
	varying vec2 vIridescenceThicknessMapUv;
#endif
#ifdef USE_SHEEN_COLORMAP
	varying vec2 vSheenColorMapUv;
#endif
#ifdef USE_SHEEN_ROUGHNESSMAP
	varying vec2 vSheenRoughnessMapUv;
#endif
#ifdef USE_SPECULARMAP
	varying vec2 vSpecularMapUv;
#endif
#ifdef USE_SPECULAR_COLORMAP
	varying vec2 vSpecularColorMapUv;
#endif
#ifdef USE_SPECULAR_INTENSITYMAP
	varying vec2 vSpecularIntensityMapUv;
#endif
#ifdef USE_TRANSMISSIONMAP
	uniform mat3 transmissionMapTransform;
	varying vec2 vTransmissionMapUv;
#endif
#ifdef USE_THICKNESSMAP
	uniform mat3 thicknessMapTransform;
	varying vec2 vThicknessMapUv;
#endif`,yE=`#if defined( USE_UV ) || defined( USE_ANISOTROPY )
	varying vec2 vUv;
#endif
#ifdef USE_MAP
	uniform mat3 mapTransform;
	varying vec2 vMapUv;
#endif
#ifdef USE_ALPHAMAP
	uniform mat3 alphaMapTransform;
	varying vec2 vAlphaMapUv;
#endif
#ifdef USE_LIGHTMAP
	uniform mat3 lightMapTransform;
	varying vec2 vLightMapUv;
#endif
#ifdef USE_AOMAP
	uniform mat3 aoMapTransform;
	varying vec2 vAoMapUv;
#endif
#ifdef USE_BUMPMAP
	uniform mat3 bumpMapTransform;
	varying vec2 vBumpMapUv;
#endif
#ifdef USE_NORMALMAP
	uniform mat3 normalMapTransform;
	varying vec2 vNormalMapUv;
#endif
#ifdef USE_DISPLACEMENTMAP
	uniform mat3 displacementMapTransform;
	varying vec2 vDisplacementMapUv;
#endif
#ifdef USE_EMISSIVEMAP
	uniform mat3 emissiveMapTransform;
	varying vec2 vEmissiveMapUv;
#endif
#ifdef USE_METALNESSMAP
	uniform mat3 metalnessMapTransform;
	varying vec2 vMetalnessMapUv;
#endif
#ifdef USE_ROUGHNESSMAP
	uniform mat3 roughnessMapTransform;
	varying vec2 vRoughnessMapUv;
#endif
#ifdef USE_ANISOTROPYMAP
	uniform mat3 anisotropyMapTransform;
	varying vec2 vAnisotropyMapUv;
#endif
#ifdef USE_CLEARCOATMAP
	uniform mat3 clearcoatMapTransform;
	varying vec2 vClearcoatMapUv;
#endif
#ifdef USE_CLEARCOAT_NORMALMAP
	uniform mat3 clearcoatNormalMapTransform;
	varying vec2 vClearcoatNormalMapUv;
#endif
#ifdef USE_CLEARCOAT_ROUGHNESSMAP
	uniform mat3 clearcoatRoughnessMapTransform;
	varying vec2 vClearcoatRoughnessMapUv;
#endif
#ifdef USE_SHEEN_COLORMAP
	uniform mat3 sheenColorMapTransform;
	varying vec2 vSheenColorMapUv;
#endif
#ifdef USE_SHEEN_ROUGHNESSMAP
	uniform mat3 sheenRoughnessMapTransform;
	varying vec2 vSheenRoughnessMapUv;
#endif
#ifdef USE_IRIDESCENCEMAP
	uniform mat3 iridescenceMapTransform;
	varying vec2 vIridescenceMapUv;
#endif
#ifdef USE_IRIDESCENCE_THICKNESSMAP
	uniform mat3 iridescenceThicknessMapTransform;
	varying vec2 vIridescenceThicknessMapUv;
#endif
#ifdef USE_SPECULARMAP
	uniform mat3 specularMapTransform;
	varying vec2 vSpecularMapUv;
#endif
#ifdef USE_SPECULAR_COLORMAP
	uniform mat3 specularColorMapTransform;
	varying vec2 vSpecularColorMapUv;
#endif
#ifdef USE_SPECULAR_INTENSITYMAP
	uniform mat3 specularIntensityMapTransform;
	varying vec2 vSpecularIntensityMapUv;
#endif
#ifdef USE_TRANSMISSIONMAP
	uniform mat3 transmissionMapTransform;
	varying vec2 vTransmissionMapUv;
#endif
#ifdef USE_THICKNESSMAP
	uniform mat3 thicknessMapTransform;
	varying vec2 vThicknessMapUv;
#endif`,SE=`#if defined( USE_UV ) || defined( USE_ANISOTROPY )
	vUv = vec3( uv, 1 ).xy;
#endif
#ifdef USE_MAP
	vMapUv = ( mapTransform * vec3( MAP_UV, 1 ) ).xy;
#endif
#ifdef USE_ALPHAMAP
	vAlphaMapUv = ( alphaMapTransform * vec3( ALPHAMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_LIGHTMAP
	vLightMapUv = ( lightMapTransform * vec3( LIGHTMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_AOMAP
	vAoMapUv = ( aoMapTransform * vec3( AOMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_BUMPMAP
	vBumpMapUv = ( bumpMapTransform * vec3( BUMPMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_NORMALMAP
	vNormalMapUv = ( normalMapTransform * vec3( NORMALMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_DISPLACEMENTMAP
	vDisplacementMapUv = ( displacementMapTransform * vec3( DISPLACEMENTMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_EMISSIVEMAP
	vEmissiveMapUv = ( emissiveMapTransform * vec3( EMISSIVEMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_METALNESSMAP
	vMetalnessMapUv = ( metalnessMapTransform * vec3( METALNESSMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_ROUGHNESSMAP
	vRoughnessMapUv = ( roughnessMapTransform * vec3( ROUGHNESSMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_ANISOTROPYMAP
	vAnisotropyMapUv = ( anisotropyMapTransform * vec3( ANISOTROPYMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_CLEARCOATMAP
	vClearcoatMapUv = ( clearcoatMapTransform * vec3( CLEARCOATMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_CLEARCOAT_NORMALMAP
	vClearcoatNormalMapUv = ( clearcoatNormalMapTransform * vec3( CLEARCOAT_NORMALMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_CLEARCOAT_ROUGHNESSMAP
	vClearcoatRoughnessMapUv = ( clearcoatRoughnessMapTransform * vec3( CLEARCOAT_ROUGHNESSMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_IRIDESCENCEMAP
	vIridescenceMapUv = ( iridescenceMapTransform * vec3( IRIDESCENCEMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_IRIDESCENCE_THICKNESSMAP
	vIridescenceThicknessMapUv = ( iridescenceThicknessMapTransform * vec3( IRIDESCENCE_THICKNESSMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_SHEEN_COLORMAP
	vSheenColorMapUv = ( sheenColorMapTransform * vec3( SHEEN_COLORMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_SHEEN_ROUGHNESSMAP
	vSheenRoughnessMapUv = ( sheenRoughnessMapTransform * vec3( SHEEN_ROUGHNESSMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_SPECULARMAP
	vSpecularMapUv = ( specularMapTransform * vec3( SPECULARMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_SPECULAR_COLORMAP
	vSpecularColorMapUv = ( specularColorMapTransform * vec3( SPECULAR_COLORMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_SPECULAR_INTENSITYMAP
	vSpecularIntensityMapUv = ( specularIntensityMapTransform * vec3( SPECULAR_INTENSITYMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_TRANSMISSIONMAP
	vTransmissionMapUv = ( transmissionMapTransform * vec3( TRANSMISSIONMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_THICKNESSMAP
	vThicknessMapUv = ( thicknessMapTransform * vec3( THICKNESSMAP_UV, 1 ) ).xy;
#endif`,bE=`#if defined( USE_ENVMAP ) || defined( DISTANCE ) || defined ( USE_SHADOWMAP ) || defined ( USE_TRANSMISSION ) || NUM_SPOT_LIGHT_COORDS > 0
	vec4 worldPosition = vec4( transformed, 1.0 );
	#ifdef USE_BATCHING
		worldPosition = batchingMatrix * worldPosition;
	#endif
	#ifdef USE_INSTANCING
		worldPosition = instanceMatrix * worldPosition;
	#endif
	worldPosition = modelMatrix * worldPosition;
#endif`;const ME=`varying vec2 vUv;
uniform mat3 uvTransform;
void main() {
	vUv = ( uvTransform * vec3( uv, 1 ) ).xy;
	gl_Position = vec4( position.xy, 1.0, 1.0 );
}`,EE=`uniform sampler2D t2D;
uniform float backgroundIntensity;
varying vec2 vUv;
void main() {
	vec4 texColor = texture2D( t2D, vUv );
	#ifdef DECODE_VIDEO_TEXTURE
		texColor = vec4( mix( pow( texColor.rgb * 0.9478672986 + vec3( 0.0521327014 ), vec3( 2.4 ) ), texColor.rgb * 0.0773993808, vec3( lessThanEqual( texColor.rgb, vec3( 0.04045 ) ) ) ), texColor.w );
	#endif
	texColor.rgb *= backgroundIntensity;
	gl_FragColor = texColor;
	#include <tonemapping_fragment>
	#include <colorspace_fragment>
}`,TE=`varying vec3 vWorldDirection;
#include <common>
void main() {
	vWorldDirection = transformDirection( position, modelMatrix );
	#include <begin_vertex>
	#include <project_vertex>
	gl_Position.z = gl_Position.w;
}`,AE=`#ifdef ENVMAP_TYPE_CUBE
	uniform samplerCube envMap;
#elif defined( ENVMAP_TYPE_CUBE_UV )
	uniform sampler2D envMap;
#endif
uniform float flipEnvMap;
uniform float backgroundBlurriness;
uniform float backgroundIntensity;
uniform mat3 backgroundRotation;
varying vec3 vWorldDirection;
#include <cube_uv_reflection_fragment>
void main() {
	#ifdef ENVMAP_TYPE_CUBE
		vec4 texColor = textureCube( envMap, backgroundRotation * vec3( flipEnvMap * vWorldDirection.x, vWorldDirection.yz ) );
	#elif defined( ENVMAP_TYPE_CUBE_UV )
		vec4 texColor = textureCubeUV( envMap, backgroundRotation * vWorldDirection, backgroundBlurriness );
	#else
		vec4 texColor = vec4( 0.0, 0.0, 0.0, 1.0 );
	#endif
	texColor.rgb *= backgroundIntensity;
	gl_FragColor = texColor;
	#include <tonemapping_fragment>
	#include <colorspace_fragment>
}`,RE=`varying vec3 vWorldDirection;
#include <common>
void main() {
	vWorldDirection = transformDirection( position, modelMatrix );
	#include <begin_vertex>
	#include <project_vertex>
	gl_Position.z = gl_Position.w;
}`,wE=`uniform samplerCube tCube;
uniform float tFlip;
uniform float opacity;
varying vec3 vWorldDirection;
void main() {
	vec4 texColor = textureCube( tCube, vec3( tFlip * vWorldDirection.x, vWorldDirection.yz ) );
	gl_FragColor = texColor;
	gl_FragColor.a *= opacity;
	#include <tonemapping_fragment>
	#include <colorspace_fragment>
}`,CE=`#include <common>
#include <batching_pars_vertex>
#include <uv_pars_vertex>
#include <displacementmap_pars_vertex>
#include <morphtarget_pars_vertex>
#include <skinning_pars_vertex>
#include <logdepthbuf_pars_vertex>
#include <clipping_planes_pars_vertex>
varying vec2 vHighPrecisionZW;
void main() {
	#include <uv_vertex>
	#include <batching_vertex>
	#include <skinbase_vertex>
	#include <morphinstance_vertex>
	#ifdef USE_DISPLACEMENTMAP
		#include <beginnormal_vertex>
		#include <morphnormal_vertex>
		#include <skinnormal_vertex>
	#endif
	#include <begin_vertex>
	#include <morphtarget_vertex>
	#include <skinning_vertex>
	#include <displacementmap_vertex>
	#include <project_vertex>
	#include <logdepthbuf_vertex>
	#include <clipping_planes_vertex>
	vHighPrecisionZW = gl_Position.zw;
}`,DE=`#if DEPTH_PACKING == 3200
	uniform float opacity;
#endif
#include <common>
#include <packing>
#include <uv_pars_fragment>
#include <map_pars_fragment>
#include <alphamap_pars_fragment>
#include <alphatest_pars_fragment>
#include <alphahash_pars_fragment>
#include <logdepthbuf_pars_fragment>
#include <clipping_planes_pars_fragment>
varying vec2 vHighPrecisionZW;
void main() {
	vec4 diffuseColor = vec4( 1.0 );
	#include <clipping_planes_fragment>
	#if DEPTH_PACKING == 3200
		diffuseColor.a = opacity;
	#endif
	#include <map_fragment>
	#include <alphamap_fragment>
	#include <alphatest_fragment>
	#include <alphahash_fragment>
	#include <logdepthbuf_fragment>
	#ifdef USE_REVERSED_DEPTH_BUFFER
		float fragCoordZ = vHighPrecisionZW[ 0 ] / vHighPrecisionZW[ 1 ];
	#else
		float fragCoordZ = 0.5 * vHighPrecisionZW[ 0 ] / vHighPrecisionZW[ 1 ] + 0.5;
	#endif
	#if DEPTH_PACKING == 3200
		gl_FragColor = vec4( vec3( 1.0 - fragCoordZ ), opacity );
	#elif DEPTH_PACKING == 3201
		gl_FragColor = packDepthToRGBA( fragCoordZ );
	#elif DEPTH_PACKING == 3202
		gl_FragColor = vec4( packDepthToRGB( fragCoordZ ), 1.0 );
	#elif DEPTH_PACKING == 3203
		gl_FragColor = vec4( packDepthToRG( fragCoordZ ), 0.0, 1.0 );
	#endif
}`,NE=`#define DISTANCE
varying vec3 vWorldPosition;
#include <common>
#include <batching_pars_vertex>
#include <uv_pars_vertex>
#include <displacementmap_pars_vertex>
#include <morphtarget_pars_vertex>
#include <skinning_pars_vertex>
#include <clipping_planes_pars_vertex>
void main() {
	#include <uv_vertex>
	#include <batching_vertex>
	#include <skinbase_vertex>
	#include <morphinstance_vertex>
	#ifdef USE_DISPLACEMENTMAP
		#include <beginnormal_vertex>
		#include <morphnormal_vertex>
		#include <skinnormal_vertex>
	#endif
	#include <begin_vertex>
	#include <morphtarget_vertex>
	#include <skinning_vertex>
	#include <displacementmap_vertex>
	#include <project_vertex>
	#include <worldpos_vertex>
	#include <clipping_planes_vertex>
	vWorldPosition = worldPosition.xyz;
}`,UE=`#define DISTANCE
uniform vec3 referencePosition;
uniform float nearDistance;
uniform float farDistance;
varying vec3 vWorldPosition;
#include <common>
#include <uv_pars_fragment>
#include <map_pars_fragment>
#include <alphamap_pars_fragment>
#include <alphatest_pars_fragment>
#include <alphahash_pars_fragment>
#include <clipping_planes_pars_fragment>
void main () {
	vec4 diffuseColor = vec4( 1.0 );
	#include <clipping_planes_fragment>
	#include <map_fragment>
	#include <alphamap_fragment>
	#include <alphatest_fragment>
	#include <alphahash_fragment>
	float dist = length( vWorldPosition - referencePosition );
	dist = ( dist - nearDistance ) / ( farDistance - nearDistance );
	dist = saturate( dist );
	gl_FragColor = vec4( dist, 0.0, 0.0, 1.0 );
}`,LE=`varying vec3 vWorldDirection;
#include <common>
void main() {
	vWorldDirection = transformDirection( position, modelMatrix );
	#include <begin_vertex>
	#include <project_vertex>
}`,OE=`uniform sampler2D tEquirect;
varying vec3 vWorldDirection;
#include <common>
void main() {
	vec3 direction = normalize( vWorldDirection );
	vec2 sampleUV = equirectUv( direction );
	gl_FragColor = texture2D( tEquirect, sampleUV );
	#include <tonemapping_fragment>
	#include <colorspace_fragment>
}`,PE=`uniform float scale;
attribute float lineDistance;
varying float vLineDistance;
#include <common>
#include <uv_pars_vertex>
#include <color_pars_vertex>
#include <fog_pars_vertex>
#include <morphtarget_pars_vertex>
#include <logdepthbuf_pars_vertex>
#include <clipping_planes_pars_vertex>
void main() {
	vLineDistance = scale * lineDistance;
	#include <uv_vertex>
	#include <color_vertex>
	#include <morphinstance_vertex>
	#include <morphcolor_vertex>
	#include <begin_vertex>
	#include <morphtarget_vertex>
	#include <project_vertex>
	#include <logdepthbuf_vertex>
	#include <clipping_planes_vertex>
	#include <fog_vertex>
}`,IE=`uniform vec3 diffuse;
uniform float opacity;
uniform float dashSize;
uniform float totalSize;
varying float vLineDistance;
#include <common>
#include <color_pars_fragment>
#include <uv_pars_fragment>
#include <map_pars_fragment>
#include <fog_pars_fragment>
#include <logdepthbuf_pars_fragment>
#include <clipping_planes_pars_fragment>
void main() {
	vec4 diffuseColor = vec4( diffuse, opacity );
	#include <clipping_planes_fragment>
	if ( mod( vLineDistance, totalSize ) > dashSize ) {
		discard;
	}
	vec3 outgoingLight = vec3( 0.0 );
	#include <logdepthbuf_fragment>
	#include <map_fragment>
	#include <color_fragment>
	outgoingLight = diffuseColor.rgb;
	#include <opaque_fragment>
	#include <tonemapping_fragment>
	#include <colorspace_fragment>
	#include <fog_fragment>
	#include <premultiplied_alpha_fragment>
}`,FE=`#include <common>
#include <batching_pars_vertex>
#include <uv_pars_vertex>
#include <envmap_pars_vertex>
#include <color_pars_vertex>
#include <fog_pars_vertex>
#include <morphtarget_pars_vertex>
#include <skinning_pars_vertex>
#include <logdepthbuf_pars_vertex>
#include <clipping_planes_pars_vertex>
void main() {
	#include <uv_vertex>
	#include <color_vertex>
	#include <morphinstance_vertex>
	#include <morphcolor_vertex>
	#include <batching_vertex>
	#if defined ( USE_ENVMAP ) || defined ( USE_SKINNING )
		#include <beginnormal_vertex>
		#include <morphnormal_vertex>
		#include <skinbase_vertex>
		#include <skinnormal_vertex>
		#include <defaultnormal_vertex>
	#endif
	#include <begin_vertex>
	#include <morphtarget_vertex>
	#include <skinning_vertex>
	#include <project_vertex>
	#include <logdepthbuf_vertex>
	#include <clipping_planes_vertex>
	#include <worldpos_vertex>
	#include <envmap_vertex>
	#include <fog_vertex>
}`,zE=`uniform vec3 diffuse;
uniform float opacity;
#ifndef FLAT_SHADED
	varying vec3 vNormal;
#endif
#include <common>
#include <dithering_pars_fragment>
#include <color_pars_fragment>
#include <uv_pars_fragment>
#include <map_pars_fragment>
#include <alphamap_pars_fragment>
#include <alphatest_pars_fragment>
#include <alphahash_pars_fragment>
#include <aomap_pars_fragment>
#include <lightmap_pars_fragment>
#include <envmap_common_pars_fragment>
#include <envmap_pars_fragment>
#include <fog_pars_fragment>
#include <specularmap_pars_fragment>
#include <logdepthbuf_pars_fragment>
#include <clipping_planes_pars_fragment>
void main() {
	vec4 diffuseColor = vec4( diffuse, opacity );
	#include <clipping_planes_fragment>
	#include <logdepthbuf_fragment>
	#include <map_fragment>
	#include <color_fragment>
	#include <alphamap_fragment>
	#include <alphatest_fragment>
	#include <alphahash_fragment>
	#include <specularmap_fragment>
	ReflectedLight reflectedLight = ReflectedLight( vec3( 0.0 ), vec3( 0.0 ), vec3( 0.0 ), vec3( 0.0 ) );
	#ifdef USE_LIGHTMAP
		vec4 lightMapTexel = texture2D( lightMap, vLightMapUv );
		reflectedLight.indirectDiffuse += lightMapTexel.rgb * lightMapIntensity * RECIPROCAL_PI;
	#else
		reflectedLight.indirectDiffuse += vec3( 1.0 );
	#endif
	#include <aomap_fragment>
	reflectedLight.indirectDiffuse *= diffuseColor.rgb;
	vec3 outgoingLight = reflectedLight.indirectDiffuse;
	#include <envmap_fragment>
	#include <opaque_fragment>
	#include <tonemapping_fragment>
	#include <colorspace_fragment>
	#include <fog_fragment>
	#include <premultiplied_alpha_fragment>
	#include <dithering_fragment>
}`,BE=`#define LAMBERT
varying vec3 vViewPosition;
#include <common>
#include <batching_pars_vertex>
#include <uv_pars_vertex>
#include <displacementmap_pars_vertex>
#include <envmap_pars_vertex>
#include <color_pars_vertex>
#include <fog_pars_vertex>
#include <normal_pars_vertex>
#include <morphtarget_pars_vertex>
#include <skinning_pars_vertex>
#include <shadowmap_pars_vertex>
#include <logdepthbuf_pars_vertex>
#include <clipping_planes_pars_vertex>
void main() {
	#include <uv_vertex>
	#include <color_vertex>
	#include <morphinstance_vertex>
	#include <morphcolor_vertex>
	#include <batching_vertex>
	#include <beginnormal_vertex>
	#include <morphnormal_vertex>
	#include <skinbase_vertex>
	#include <skinnormal_vertex>
	#include <defaultnormal_vertex>
	#include <normal_vertex>
	#include <begin_vertex>
	#include <morphtarget_vertex>
	#include <skinning_vertex>
	#include <displacementmap_vertex>
	#include <project_vertex>
	#include <logdepthbuf_vertex>
	#include <clipping_planes_vertex>
	vViewPosition = - mvPosition.xyz;
	#include <worldpos_vertex>
	#include <envmap_vertex>
	#include <shadowmap_vertex>
	#include <fog_vertex>
}`,HE=`#define LAMBERT
uniform vec3 diffuse;
uniform vec3 emissive;
uniform float opacity;
#include <common>
#include <dithering_pars_fragment>
#include <color_pars_fragment>
#include <uv_pars_fragment>
#include <map_pars_fragment>
#include <alphamap_pars_fragment>
#include <alphatest_pars_fragment>
#include <alphahash_pars_fragment>
#include <aomap_pars_fragment>
#include <lightmap_pars_fragment>
#include <emissivemap_pars_fragment>
#include <cube_uv_reflection_fragment>
#include <envmap_common_pars_fragment>
#include <envmap_pars_fragment>
#include <envmap_physical_pars_fragment>
#include <fog_pars_fragment>
#include <bsdfs>
#include <lights_pars_begin>
#include <normal_pars_fragment>
#include <lights_lambert_pars_fragment>
#include <shadowmap_pars_fragment>
#include <bumpmap_pars_fragment>
#include <normalmap_pars_fragment>
#include <specularmap_pars_fragment>
#include <logdepthbuf_pars_fragment>
#include <clipping_planes_pars_fragment>
void main() {
	vec4 diffuseColor = vec4( diffuse, opacity );
	#include <clipping_planes_fragment>
	ReflectedLight reflectedLight = ReflectedLight( vec3( 0.0 ), vec3( 0.0 ), vec3( 0.0 ), vec3( 0.0 ) );
	vec3 totalEmissiveRadiance = emissive;
	#include <logdepthbuf_fragment>
	#include <map_fragment>
	#include <color_fragment>
	#include <alphamap_fragment>
	#include <alphatest_fragment>
	#include <alphahash_fragment>
	#include <specularmap_fragment>
	#include <normal_fragment_begin>
	#include <normal_fragment_maps>
	#include <emissivemap_fragment>
	#include <lights_lambert_fragment>
	#include <lights_fragment_begin>
	#include <lights_fragment_maps>
	#include <lights_fragment_end>
	#include <aomap_fragment>
	vec3 outgoingLight = reflectedLight.directDiffuse + reflectedLight.indirectDiffuse + totalEmissiveRadiance;
	#include <envmap_fragment>
	#include <opaque_fragment>
	#include <tonemapping_fragment>
	#include <colorspace_fragment>
	#include <fog_fragment>
	#include <premultiplied_alpha_fragment>
	#include <dithering_fragment>
}`,GE=`#define MATCAP
varying vec3 vViewPosition;
#include <common>
#include <batching_pars_vertex>
#include <uv_pars_vertex>
#include <color_pars_vertex>
#include <displacementmap_pars_vertex>
#include <fog_pars_vertex>
#include <normal_pars_vertex>
#include <morphtarget_pars_vertex>
#include <skinning_pars_vertex>
#include <logdepthbuf_pars_vertex>
#include <clipping_planes_pars_vertex>
void main() {
	#include <uv_vertex>
	#include <color_vertex>
	#include <morphinstance_vertex>
	#include <morphcolor_vertex>
	#include <batching_vertex>
	#include <beginnormal_vertex>
	#include <morphnormal_vertex>
	#include <skinbase_vertex>
	#include <skinnormal_vertex>
	#include <defaultnormal_vertex>
	#include <normal_vertex>
	#include <begin_vertex>
	#include <morphtarget_vertex>
	#include <skinning_vertex>
	#include <displacementmap_vertex>
	#include <project_vertex>
	#include <logdepthbuf_vertex>
	#include <clipping_planes_vertex>
	#include <fog_vertex>
	vViewPosition = - mvPosition.xyz;
}`,VE=`#define MATCAP
uniform vec3 diffuse;
uniform float opacity;
uniform sampler2D matcap;
varying vec3 vViewPosition;
#include <common>
#include <dithering_pars_fragment>
#include <color_pars_fragment>
#include <uv_pars_fragment>
#include <map_pars_fragment>
#include <alphamap_pars_fragment>
#include <alphatest_pars_fragment>
#include <alphahash_pars_fragment>
#include <fog_pars_fragment>
#include <normal_pars_fragment>
#include <bumpmap_pars_fragment>
#include <normalmap_pars_fragment>
#include <logdepthbuf_pars_fragment>
#include <clipping_planes_pars_fragment>
void main() {
	vec4 diffuseColor = vec4( diffuse, opacity );
	#include <clipping_planes_fragment>
	#include <logdepthbuf_fragment>
	#include <map_fragment>
	#include <color_fragment>
	#include <alphamap_fragment>
	#include <alphatest_fragment>
	#include <alphahash_fragment>
	#include <normal_fragment_begin>
	#include <normal_fragment_maps>
	vec3 viewDir = normalize( vViewPosition );
	vec3 x = normalize( vec3( viewDir.z, 0.0, - viewDir.x ) );
	vec3 y = cross( viewDir, x );
	vec2 uv = vec2( dot( x, normal ), dot( y, normal ) ) * 0.495 + 0.5;
	#ifdef USE_MATCAP
		vec4 matcapColor = texture2D( matcap, uv );
	#else
		vec4 matcapColor = vec4( vec3( mix( 0.2, 0.8, uv.y ) ), 1.0 );
	#endif
	vec3 outgoingLight = diffuseColor.rgb * matcapColor.rgb;
	#include <opaque_fragment>
	#include <tonemapping_fragment>
	#include <colorspace_fragment>
	#include <fog_fragment>
	#include <premultiplied_alpha_fragment>
	#include <dithering_fragment>
}`,kE=`#define NORMAL
#if defined( FLAT_SHADED ) || defined( USE_BUMPMAP ) || defined( USE_NORMALMAP_TANGENTSPACE )
	varying vec3 vViewPosition;
#endif
#include <common>
#include <batching_pars_vertex>
#include <uv_pars_vertex>
#include <displacementmap_pars_vertex>
#include <normal_pars_vertex>
#include <morphtarget_pars_vertex>
#include <skinning_pars_vertex>
#include <logdepthbuf_pars_vertex>
#include <clipping_planes_pars_vertex>
void main() {
	#include <uv_vertex>
	#include <batching_vertex>
	#include <beginnormal_vertex>
	#include <morphinstance_vertex>
	#include <morphnormal_vertex>
	#include <skinbase_vertex>
	#include <skinnormal_vertex>
	#include <defaultnormal_vertex>
	#include <normal_vertex>
	#include <begin_vertex>
	#include <morphtarget_vertex>
	#include <skinning_vertex>
	#include <displacementmap_vertex>
	#include <project_vertex>
	#include <logdepthbuf_vertex>
	#include <clipping_planes_vertex>
#if defined( FLAT_SHADED ) || defined( USE_BUMPMAP ) || defined( USE_NORMALMAP_TANGENTSPACE )
	vViewPosition = - mvPosition.xyz;
#endif
}`,XE=`#define NORMAL
uniform float opacity;
#if defined( FLAT_SHADED ) || defined( USE_BUMPMAP ) || defined( USE_NORMALMAP_TANGENTSPACE )
	varying vec3 vViewPosition;
#endif
#include <uv_pars_fragment>
#include <normal_pars_fragment>
#include <bumpmap_pars_fragment>
#include <normalmap_pars_fragment>
#include <logdepthbuf_pars_fragment>
#include <clipping_planes_pars_fragment>
void main() {
	vec4 diffuseColor = vec4( 0.0, 0.0, 0.0, opacity );
	#include <clipping_planes_fragment>
	#include <logdepthbuf_fragment>
	#include <normal_fragment_begin>
	#include <normal_fragment_maps>
	gl_FragColor = vec4( normalize( normal ) * 0.5 + 0.5, diffuseColor.a );
	#ifdef OPAQUE
		gl_FragColor.a = 1.0;
	#endif
}`,jE=`#define PHONG
varying vec3 vViewPosition;
#include <common>
#include <batching_pars_vertex>
#include <uv_pars_vertex>
#include <displacementmap_pars_vertex>
#include <envmap_pars_vertex>
#include <color_pars_vertex>
#include <fog_pars_vertex>
#include <normal_pars_vertex>
#include <morphtarget_pars_vertex>
#include <skinning_pars_vertex>
#include <shadowmap_pars_vertex>
#include <logdepthbuf_pars_vertex>
#include <clipping_planes_pars_vertex>
void main() {
	#include <uv_vertex>
	#include <color_vertex>
	#include <morphcolor_vertex>
	#include <batching_vertex>
	#include <beginnormal_vertex>
	#include <morphinstance_vertex>
	#include <morphnormal_vertex>
	#include <skinbase_vertex>
	#include <skinnormal_vertex>
	#include <defaultnormal_vertex>
	#include <normal_vertex>
	#include <begin_vertex>
	#include <morphtarget_vertex>
	#include <skinning_vertex>
	#include <displacementmap_vertex>
	#include <project_vertex>
	#include <logdepthbuf_vertex>
	#include <clipping_planes_vertex>
	vViewPosition = - mvPosition.xyz;
	#include <worldpos_vertex>
	#include <envmap_vertex>
	#include <shadowmap_vertex>
	#include <fog_vertex>
}`,WE=`#define PHONG
uniform vec3 diffuse;
uniform vec3 emissive;
uniform vec3 specular;
uniform float shininess;
uniform float opacity;
#include <common>
#include <dithering_pars_fragment>
#include <color_pars_fragment>
#include <uv_pars_fragment>
#include <map_pars_fragment>
#include <alphamap_pars_fragment>
#include <alphatest_pars_fragment>
#include <alphahash_pars_fragment>
#include <aomap_pars_fragment>
#include <lightmap_pars_fragment>
#include <emissivemap_pars_fragment>
#include <cube_uv_reflection_fragment>
#include <envmap_common_pars_fragment>
#include <envmap_pars_fragment>
#include <envmap_physical_pars_fragment>
#include <fog_pars_fragment>
#include <bsdfs>
#include <lights_pars_begin>
#include <normal_pars_fragment>
#include <lights_phong_pars_fragment>
#include <shadowmap_pars_fragment>
#include <bumpmap_pars_fragment>
#include <normalmap_pars_fragment>
#include <specularmap_pars_fragment>
#include <logdepthbuf_pars_fragment>
#include <clipping_planes_pars_fragment>
void main() {
	vec4 diffuseColor = vec4( diffuse, opacity );
	#include <clipping_planes_fragment>
	ReflectedLight reflectedLight = ReflectedLight( vec3( 0.0 ), vec3( 0.0 ), vec3( 0.0 ), vec3( 0.0 ) );
	vec3 totalEmissiveRadiance = emissive;
	#include <logdepthbuf_fragment>
	#include <map_fragment>
	#include <color_fragment>
	#include <alphamap_fragment>
	#include <alphatest_fragment>
	#include <alphahash_fragment>
	#include <specularmap_fragment>
	#include <normal_fragment_begin>
	#include <normal_fragment_maps>
	#include <emissivemap_fragment>
	#include <lights_phong_fragment>
	#include <lights_fragment_begin>
	#include <lights_fragment_maps>
	#include <lights_fragment_end>
	#include <aomap_fragment>
	vec3 outgoingLight = reflectedLight.directDiffuse + reflectedLight.indirectDiffuse + reflectedLight.directSpecular + reflectedLight.indirectSpecular + totalEmissiveRadiance;
	#include <envmap_fragment>
	#include <opaque_fragment>
	#include <tonemapping_fragment>
	#include <colorspace_fragment>
	#include <fog_fragment>
	#include <premultiplied_alpha_fragment>
	#include <dithering_fragment>
}`,qE=`#define STANDARD
varying vec3 vViewPosition;
#ifdef USE_TRANSMISSION
	varying vec3 vWorldPosition;
#endif
#include <common>
#include <batching_pars_vertex>
#include <uv_pars_vertex>
#include <displacementmap_pars_vertex>
#include <color_pars_vertex>
#include <fog_pars_vertex>
#include <normal_pars_vertex>
#include <morphtarget_pars_vertex>
#include <skinning_pars_vertex>
#include <shadowmap_pars_vertex>
#include <logdepthbuf_pars_vertex>
#include <clipping_planes_pars_vertex>
void main() {
	#include <uv_vertex>
	#include <color_vertex>
	#include <morphinstance_vertex>
	#include <morphcolor_vertex>
	#include <batching_vertex>
	#include <beginnormal_vertex>
	#include <morphnormal_vertex>
	#include <skinbase_vertex>
	#include <skinnormal_vertex>
	#include <defaultnormal_vertex>
	#include <normal_vertex>
	#include <begin_vertex>
	#include <morphtarget_vertex>
	#include <skinning_vertex>
	#include <displacementmap_vertex>
	#include <project_vertex>
	#include <logdepthbuf_vertex>
	#include <clipping_planes_vertex>
	vViewPosition = - mvPosition.xyz;
	#include <worldpos_vertex>
	#include <shadowmap_vertex>
	#include <fog_vertex>
#ifdef USE_TRANSMISSION
	vWorldPosition = worldPosition.xyz;
#endif
}`,YE=`#define STANDARD
#ifdef PHYSICAL
	#define IOR
	#define USE_SPECULAR
#endif
uniform vec3 diffuse;
uniform vec3 emissive;
uniform float roughness;
uniform float metalness;
uniform float opacity;
#ifdef IOR
	uniform float ior;
#endif
#ifdef USE_SPECULAR
	uniform float specularIntensity;
	uniform vec3 specularColor;
	#ifdef USE_SPECULAR_COLORMAP
		uniform sampler2D specularColorMap;
	#endif
	#ifdef USE_SPECULAR_INTENSITYMAP
		uniform sampler2D specularIntensityMap;
	#endif
#endif
#ifdef USE_CLEARCOAT
	uniform float clearcoat;
	uniform float clearcoatRoughness;
#endif
#ifdef USE_DISPERSION
	uniform float dispersion;
#endif
#ifdef USE_IRIDESCENCE
	uniform float iridescence;
	uniform float iridescenceIOR;
	uniform float iridescenceThicknessMinimum;
	uniform float iridescenceThicknessMaximum;
#endif
#ifdef USE_SHEEN
	uniform vec3 sheenColor;
	uniform float sheenRoughness;
	#ifdef USE_SHEEN_COLORMAP
		uniform sampler2D sheenColorMap;
	#endif
	#ifdef USE_SHEEN_ROUGHNESSMAP
		uniform sampler2D sheenRoughnessMap;
	#endif
#endif
#ifdef USE_ANISOTROPY
	uniform vec2 anisotropyVector;
	#ifdef USE_ANISOTROPYMAP
		uniform sampler2D anisotropyMap;
	#endif
#endif
varying vec3 vViewPosition;
#include <common>
#include <dithering_pars_fragment>
#include <color_pars_fragment>
#include <uv_pars_fragment>
#include <map_pars_fragment>
#include <alphamap_pars_fragment>
#include <alphatest_pars_fragment>
#include <alphahash_pars_fragment>
#include <aomap_pars_fragment>
#include <lightmap_pars_fragment>
#include <emissivemap_pars_fragment>
#include <iridescence_fragment>
#include <cube_uv_reflection_fragment>
#include <envmap_common_pars_fragment>
#include <envmap_physical_pars_fragment>
#include <fog_pars_fragment>
#include <lights_pars_begin>
#include <normal_pars_fragment>
#include <lights_physical_pars_fragment>
#include <transmission_pars_fragment>
#include <shadowmap_pars_fragment>
#include <bumpmap_pars_fragment>
#include <normalmap_pars_fragment>
#include <clearcoat_pars_fragment>
#include <iridescence_pars_fragment>
#include <roughnessmap_pars_fragment>
#include <metalnessmap_pars_fragment>
#include <logdepthbuf_pars_fragment>
#include <clipping_planes_pars_fragment>
void main() {
	vec4 diffuseColor = vec4( diffuse, opacity );
	#include <clipping_planes_fragment>
	ReflectedLight reflectedLight = ReflectedLight( vec3( 0.0 ), vec3( 0.0 ), vec3( 0.0 ), vec3( 0.0 ) );
	vec3 totalEmissiveRadiance = emissive;
	#include <logdepthbuf_fragment>
	#include <map_fragment>
	#include <color_fragment>
	#include <alphamap_fragment>
	#include <alphatest_fragment>
	#include <alphahash_fragment>
	#include <roughnessmap_fragment>
	#include <metalnessmap_fragment>
	#include <normal_fragment_begin>
	#include <normal_fragment_maps>
	#include <clearcoat_normal_fragment_begin>
	#include <clearcoat_normal_fragment_maps>
	#include <emissivemap_fragment>
	#include <lights_physical_fragment>
	#include <lights_fragment_begin>
	#include <lights_fragment_maps>
	#include <lights_fragment_end>
	#include <aomap_fragment>
	vec3 totalDiffuse = reflectedLight.directDiffuse + reflectedLight.indirectDiffuse;
	vec3 totalSpecular = reflectedLight.directSpecular + reflectedLight.indirectSpecular;
	#include <transmission_fragment>
	vec3 outgoingLight = totalDiffuse + totalSpecular + totalEmissiveRadiance;
	#ifdef USE_SHEEN
 
		outgoingLight = outgoingLight + sheenSpecularDirect + sheenSpecularIndirect;
 
 	#endif
	#ifdef USE_CLEARCOAT
		float dotNVcc = saturate( dot( geometryClearcoatNormal, geometryViewDir ) );
		vec3 Fcc = F_Schlick( material.clearcoatF0, material.clearcoatF90, dotNVcc );
		outgoingLight = outgoingLight * ( 1.0 - material.clearcoat * Fcc ) + ( clearcoatSpecularDirect + clearcoatSpecularIndirect ) * material.clearcoat;
	#endif
	#include <opaque_fragment>
	#include <tonemapping_fragment>
	#include <colorspace_fragment>
	#include <fog_fragment>
	#include <premultiplied_alpha_fragment>
	#include <dithering_fragment>
}`,ZE=`#define TOON
varying vec3 vViewPosition;
#include <common>
#include <batching_pars_vertex>
#include <uv_pars_vertex>
#include <displacementmap_pars_vertex>
#include <color_pars_vertex>
#include <fog_pars_vertex>
#include <normal_pars_vertex>
#include <morphtarget_pars_vertex>
#include <skinning_pars_vertex>
#include <shadowmap_pars_vertex>
#include <logdepthbuf_pars_vertex>
#include <clipping_planes_pars_vertex>
void main() {
	#include <uv_vertex>
	#include <color_vertex>
	#include <morphinstance_vertex>
	#include <morphcolor_vertex>
	#include <batching_vertex>
	#include <beginnormal_vertex>
	#include <morphnormal_vertex>
	#include <skinbase_vertex>
	#include <skinnormal_vertex>
	#include <defaultnormal_vertex>
	#include <normal_vertex>
	#include <begin_vertex>
	#include <morphtarget_vertex>
	#include <skinning_vertex>
	#include <displacementmap_vertex>
	#include <project_vertex>
	#include <logdepthbuf_vertex>
	#include <clipping_planes_vertex>
	vViewPosition = - mvPosition.xyz;
	#include <worldpos_vertex>
	#include <shadowmap_vertex>
	#include <fog_vertex>
}`,KE=`#define TOON
uniform vec3 diffuse;
uniform vec3 emissive;
uniform float opacity;
#include <common>
#include <dithering_pars_fragment>
#include <color_pars_fragment>
#include <uv_pars_fragment>
#include <map_pars_fragment>
#include <alphamap_pars_fragment>
#include <alphatest_pars_fragment>
#include <alphahash_pars_fragment>
#include <aomap_pars_fragment>
#include <lightmap_pars_fragment>
#include <emissivemap_pars_fragment>
#include <gradientmap_pars_fragment>
#include <fog_pars_fragment>
#include <bsdfs>
#include <lights_pars_begin>
#include <normal_pars_fragment>
#include <lights_toon_pars_fragment>
#include <shadowmap_pars_fragment>
#include <bumpmap_pars_fragment>
#include <normalmap_pars_fragment>
#include <logdepthbuf_pars_fragment>
#include <clipping_planes_pars_fragment>
void main() {
	vec4 diffuseColor = vec4( diffuse, opacity );
	#include <clipping_planes_fragment>
	ReflectedLight reflectedLight = ReflectedLight( vec3( 0.0 ), vec3( 0.0 ), vec3( 0.0 ), vec3( 0.0 ) );
	vec3 totalEmissiveRadiance = emissive;
	#include <logdepthbuf_fragment>
	#include <map_fragment>
	#include <color_fragment>
	#include <alphamap_fragment>
	#include <alphatest_fragment>
	#include <alphahash_fragment>
	#include <normal_fragment_begin>
	#include <normal_fragment_maps>
	#include <emissivemap_fragment>
	#include <lights_toon_fragment>
	#include <lights_fragment_begin>
	#include <lights_fragment_maps>
	#include <lights_fragment_end>
	#include <aomap_fragment>
	vec3 outgoingLight = reflectedLight.directDiffuse + reflectedLight.indirectDiffuse + totalEmissiveRadiance;
	#include <opaque_fragment>
	#include <tonemapping_fragment>
	#include <colorspace_fragment>
	#include <fog_fragment>
	#include <premultiplied_alpha_fragment>
	#include <dithering_fragment>
}`,QE=`uniform float size;
uniform float scale;
#include <common>
#include <color_pars_vertex>
#include <fog_pars_vertex>
#include <morphtarget_pars_vertex>
#include <logdepthbuf_pars_vertex>
#include <clipping_planes_pars_vertex>
#ifdef USE_POINTS_UV
	varying vec2 vUv;
	uniform mat3 uvTransform;
#endif
void main() {
	#ifdef USE_POINTS_UV
		vUv = ( uvTransform * vec3( uv, 1 ) ).xy;
	#endif
	#include <color_vertex>
	#include <morphinstance_vertex>
	#include <morphcolor_vertex>
	#include <begin_vertex>
	#include <morphtarget_vertex>
	#include <project_vertex>
	gl_PointSize = size;
	#ifdef USE_SIZEATTENUATION
		bool isPerspective = isPerspectiveMatrix( projectionMatrix );
		if ( isPerspective ) gl_PointSize *= ( scale / - mvPosition.z );
	#endif
	#include <logdepthbuf_vertex>
	#include <clipping_planes_vertex>
	#include <worldpos_vertex>
	#include <fog_vertex>
}`,JE=`uniform vec3 diffuse;
uniform float opacity;
#include <common>
#include <color_pars_fragment>
#include <map_particle_pars_fragment>
#include <alphatest_pars_fragment>
#include <alphahash_pars_fragment>
#include <fog_pars_fragment>
#include <logdepthbuf_pars_fragment>
#include <clipping_planes_pars_fragment>
void main() {
	vec4 diffuseColor = vec4( diffuse, opacity );
	#include <clipping_planes_fragment>
	vec3 outgoingLight = vec3( 0.0 );
	#include <logdepthbuf_fragment>
	#include <map_particle_fragment>
	#include <color_fragment>
	#include <alphatest_fragment>
	#include <alphahash_fragment>
	outgoingLight = diffuseColor.rgb;
	#include <opaque_fragment>
	#include <tonemapping_fragment>
	#include <colorspace_fragment>
	#include <fog_fragment>
	#include <premultiplied_alpha_fragment>
}`,$E=`#include <common>
#include <batching_pars_vertex>
#include <fog_pars_vertex>
#include <morphtarget_pars_vertex>
#include <skinning_pars_vertex>
#include <logdepthbuf_pars_vertex>
#include <shadowmap_pars_vertex>
void main() {
	#include <batching_vertex>
	#include <beginnormal_vertex>
	#include <morphinstance_vertex>
	#include <morphnormal_vertex>
	#include <skinbase_vertex>
	#include <skinnormal_vertex>
	#include <defaultnormal_vertex>
	#include <begin_vertex>
	#include <morphtarget_vertex>
	#include <skinning_vertex>
	#include <project_vertex>
	#include <logdepthbuf_vertex>
	#include <worldpos_vertex>
	#include <shadowmap_vertex>
	#include <fog_vertex>
}`,eT=`uniform vec3 color;
uniform float opacity;
#include <common>
#include <fog_pars_fragment>
#include <bsdfs>
#include <lights_pars_begin>
#include <logdepthbuf_pars_fragment>
#include <shadowmap_pars_fragment>
#include <shadowmask_pars_fragment>
void main() {
	#include <logdepthbuf_fragment>
	gl_FragColor = vec4( color, opacity * ( 1.0 - getShadowMask() ) );
	#include <tonemapping_fragment>
	#include <colorspace_fragment>
	#include <fog_fragment>
	#include <premultiplied_alpha_fragment>
}`,tT=`uniform float rotation;
uniform vec2 center;
#include <common>
#include <uv_pars_vertex>
#include <fog_pars_vertex>
#include <logdepthbuf_pars_vertex>
#include <clipping_planes_pars_vertex>
void main() {
	#include <uv_vertex>
	vec4 mvPosition = modelViewMatrix[ 3 ];
	vec2 scale = vec2( length( modelMatrix[ 0 ].xyz ), length( modelMatrix[ 1 ].xyz ) );
	#ifndef USE_SIZEATTENUATION
		bool isPerspective = isPerspectiveMatrix( projectionMatrix );
		if ( isPerspective ) scale *= - mvPosition.z;
	#endif
	vec2 alignedPosition = ( position.xy - ( center - vec2( 0.5 ) ) ) * scale;
	vec2 rotatedPosition;
	rotatedPosition.x = cos( rotation ) * alignedPosition.x - sin( rotation ) * alignedPosition.y;
	rotatedPosition.y = sin( rotation ) * alignedPosition.x + cos( rotation ) * alignedPosition.y;
	mvPosition.xy += rotatedPosition;
	gl_Position = projectionMatrix * mvPosition;
	#include <logdepthbuf_vertex>
	#include <clipping_planes_vertex>
	#include <fog_vertex>
}`,nT=`uniform vec3 diffuse;
uniform float opacity;
#include <common>
#include <uv_pars_fragment>
#include <map_pars_fragment>
#include <alphamap_pars_fragment>
#include <alphatest_pars_fragment>
#include <alphahash_pars_fragment>
#include <fog_pars_fragment>
#include <logdepthbuf_pars_fragment>
#include <clipping_planes_pars_fragment>
void main() {
	vec4 diffuseColor = vec4( diffuse, opacity );
	#include <clipping_planes_fragment>
	vec3 outgoingLight = vec3( 0.0 );
	#include <logdepthbuf_fragment>
	#include <map_fragment>
	#include <alphamap_fragment>
	#include <alphatest_fragment>
	#include <alphahash_fragment>
	outgoingLight = diffuseColor.rgb;
	#include <opaque_fragment>
	#include <tonemapping_fragment>
	#include <colorspace_fragment>
	#include <fog_fragment>
}`,pt={alphahash_fragment:Eb,alphahash_pars_fragment:Tb,alphamap_fragment:Ab,alphamap_pars_fragment:Rb,alphatest_fragment:wb,alphatest_pars_fragment:Cb,aomap_fragment:Db,aomap_pars_fragment:Nb,batching_pars_vertex:Ub,batching_vertex:Lb,begin_vertex:Ob,beginnormal_vertex:Pb,bsdfs:Ib,iridescence_fragment:Fb,bumpmap_pars_fragment:zb,clipping_planes_fragment:Bb,clipping_planes_pars_fragment:Hb,clipping_planes_pars_vertex:Gb,clipping_planes_vertex:Vb,color_fragment:kb,color_pars_fragment:Xb,color_pars_vertex:jb,color_vertex:Wb,common:qb,cube_uv_reflection_fragment:Yb,defaultnormal_vertex:Zb,displacementmap_pars_vertex:Kb,displacementmap_vertex:Qb,emissivemap_fragment:Jb,emissivemap_pars_fragment:$b,colorspace_fragment:eM,colorspace_pars_fragment:tM,envmap_fragment:nM,envmap_common_pars_fragment:iM,envmap_pars_fragment:aM,envmap_pars_vertex:sM,envmap_physical_pars_fragment:gM,envmap_vertex:rM,fog_vertex:oM,fog_pars_vertex:lM,fog_fragment:cM,fog_pars_fragment:uM,gradientmap_pars_fragment:fM,lightmap_pars_fragment:dM,lights_lambert_fragment:hM,lights_lambert_pars_fragment:pM,lights_pars_begin:mM,lights_toon_fragment:_M,lights_toon_pars_fragment:vM,lights_phong_fragment:xM,lights_phong_pars_fragment:yM,lights_physical_fragment:SM,lights_physical_pars_fragment:bM,lights_fragment_begin:MM,lights_fragment_maps:EM,lights_fragment_end:TM,logdepthbuf_fragment:AM,logdepthbuf_pars_fragment:RM,logdepthbuf_pars_vertex:wM,logdepthbuf_vertex:CM,map_fragment:DM,map_pars_fragment:NM,map_particle_fragment:UM,map_particle_pars_fragment:LM,metalnessmap_fragment:OM,metalnessmap_pars_fragment:PM,morphinstance_vertex:IM,morphcolor_vertex:FM,morphnormal_vertex:zM,morphtarget_pars_vertex:BM,morphtarget_vertex:HM,normal_fragment_begin:GM,normal_fragment_maps:VM,normal_pars_fragment:kM,normal_pars_vertex:XM,normal_vertex:jM,normalmap_pars_fragment:WM,clearcoat_normal_fragment_begin:qM,clearcoat_normal_fragment_maps:YM,clearcoat_pars_fragment:ZM,iridescence_pars_fragment:KM,opaque_fragment:QM,packing:JM,premultiplied_alpha_fragment:$M,project_vertex:eE,dithering_fragment:tE,dithering_pars_fragment:nE,roughnessmap_fragment:iE,roughnessmap_pars_fragment:aE,shadowmap_pars_fragment:sE,shadowmap_pars_vertex:rE,shadowmap_vertex:oE,shadowmask_pars_fragment:lE,skinbase_vertex:cE,skinning_pars_vertex:uE,skinning_vertex:fE,skinnormal_vertex:dE,specularmap_fragment:hE,specularmap_pars_fragment:pE,tonemapping_fragment:mE,tonemapping_pars_fragment:gE,transmission_fragment:_E,transmission_pars_fragment:vE,uv_pars_fragment:xE,uv_pars_vertex:yE,uv_vertex:SE,worldpos_vertex:bE,background_vert:ME,background_frag:EE,backgroundCube_vert:TE,backgroundCube_frag:AE,cube_vert:RE,cube_frag:wE,depth_vert:CE,depth_frag:DE,distance_vert:NE,distance_frag:UE,equirect_vert:LE,equirect_frag:OE,linedashed_vert:PE,linedashed_frag:IE,meshbasic_vert:FE,meshbasic_frag:zE,meshlambert_vert:BE,meshlambert_frag:HE,meshmatcap_vert:GE,meshmatcap_frag:VE,meshnormal_vert:kE,meshnormal_frag:XE,meshphong_vert:jE,meshphong_frag:WE,meshphysical_vert:qE,meshphysical_frag:YE,meshtoon_vert:ZE,meshtoon_frag:KE,points_vert:QE,points_frag:JE,shadow_vert:$E,shadow_frag:eT,sprite_vert:tT,sprite_frag:nT},Ue={common:{diffuse:{value:new At(16777215)},opacity:{value:1},map:{value:null},mapTransform:{value:new ht},alphaMap:{value:null},alphaMapTransform:{value:new ht},alphaTest:{value:0}},specularmap:{specularMap:{value:null},specularMapTransform:{value:new ht}},envmap:{envMap:{value:null},envMapRotation:{value:new ht},flipEnvMap:{value:-1},reflectivity:{value:1},ior:{value:1.5},refractionRatio:{value:.98},dfgLUT:{value:null}},aomap:{aoMap:{value:null},aoMapIntensity:{value:1},aoMapTransform:{value:new ht}},lightmap:{lightMap:{value:null},lightMapIntensity:{value:1},lightMapTransform:{value:new ht}},bumpmap:{bumpMap:{value:null},bumpMapTransform:{value:new ht},bumpScale:{value:1}},normalmap:{normalMap:{value:null},normalMapTransform:{value:new ht},normalScale:{value:new ct(1,1)}},displacementmap:{displacementMap:{value:null},displacementMapTransform:{value:new ht},displacementScale:{value:1},displacementBias:{value:0}},emissivemap:{emissiveMap:{value:null},emissiveMapTransform:{value:new ht}},metalnessmap:{metalnessMap:{value:null},metalnessMapTransform:{value:new ht}},roughnessmap:{roughnessMap:{value:null},roughnessMapTransform:{value:new ht}},gradientmap:{gradientMap:{value:null}},fog:{fogDensity:{value:25e-5},fogNear:{value:1},fogFar:{value:2e3},fogColor:{value:new At(16777215)}},lights:{ambientLightColor:{value:[]},lightProbe:{value:[]},directionalLights:{value:[],properties:{direction:{},color:{}}},directionalLightShadows:{value:[],properties:{shadowIntensity:1,shadowBias:{},shadowNormalBias:{},shadowRadius:{},shadowMapSize:{}}},directionalShadowMatrix:{value:[]},spotLights:{value:[],properties:{color:{},position:{},direction:{},distance:{},coneCos:{},penumbraCos:{},decay:{}}},spotLightShadows:{value:[],properties:{shadowIntensity:1,shadowBias:{},shadowNormalBias:{},shadowRadius:{},shadowMapSize:{}}},spotLightMap:{value:[]},spotLightMatrix:{value:[]},pointLights:{value:[],properties:{color:{},position:{},decay:{},distance:{}}},pointLightShadows:{value:[],properties:{shadowIntensity:1,shadowBias:{},shadowNormalBias:{},shadowRadius:{},shadowMapSize:{},shadowCameraNear:{},shadowCameraFar:{}}},pointShadowMatrix:{value:[]},hemisphereLights:{value:[],properties:{direction:{},skyColor:{},groundColor:{}}},rectAreaLights:{value:[],properties:{color:{},position:{},width:{},height:{}}},ltc_1:{value:null},ltc_2:{value:null}},points:{diffuse:{value:new At(16777215)},opacity:{value:1},size:{value:1},scale:{value:1},map:{value:null},alphaMap:{value:null},alphaMapTransform:{value:new ht},alphaTest:{value:0},uvTransform:{value:new ht}},sprite:{diffuse:{value:new At(16777215)},opacity:{value:1},center:{value:new ct(.5,.5)},rotation:{value:0},map:{value:null},mapTransform:{value:new ht},alphaMap:{value:null},alphaMapTransform:{value:new ht},alphaTest:{value:0}}},Bi={basic:{uniforms:In([Ue.common,Ue.specularmap,Ue.envmap,Ue.aomap,Ue.lightmap,Ue.fog]),vertexShader:pt.meshbasic_vert,fragmentShader:pt.meshbasic_frag},lambert:{uniforms:In([Ue.common,Ue.specularmap,Ue.envmap,Ue.aomap,Ue.lightmap,Ue.emissivemap,Ue.bumpmap,Ue.normalmap,Ue.displacementmap,Ue.fog,Ue.lights,{emissive:{value:new At(0)},envMapIntensity:{value:1}}]),vertexShader:pt.meshlambert_vert,fragmentShader:pt.meshlambert_frag},phong:{uniforms:In([Ue.common,Ue.specularmap,Ue.envmap,Ue.aomap,Ue.lightmap,Ue.emissivemap,Ue.bumpmap,Ue.normalmap,Ue.displacementmap,Ue.fog,Ue.lights,{emissive:{value:new At(0)},specular:{value:new At(1118481)},shininess:{value:30},envMapIntensity:{value:1}}]),vertexShader:pt.meshphong_vert,fragmentShader:pt.meshphong_frag},standard:{uniforms:In([Ue.common,Ue.envmap,Ue.aomap,Ue.lightmap,Ue.emissivemap,Ue.bumpmap,Ue.normalmap,Ue.displacementmap,Ue.roughnessmap,Ue.metalnessmap,Ue.fog,Ue.lights,{emissive:{value:new At(0)},roughness:{value:1},metalness:{value:0},envMapIntensity:{value:1}}]),vertexShader:pt.meshphysical_vert,fragmentShader:pt.meshphysical_frag},toon:{uniforms:In([Ue.common,Ue.aomap,Ue.lightmap,Ue.emissivemap,Ue.bumpmap,Ue.normalmap,Ue.displacementmap,Ue.gradientmap,Ue.fog,Ue.lights,{emissive:{value:new At(0)}}]),vertexShader:pt.meshtoon_vert,fragmentShader:pt.meshtoon_frag},matcap:{uniforms:In([Ue.common,Ue.bumpmap,Ue.normalmap,Ue.displacementmap,Ue.fog,{matcap:{value:null}}]),vertexShader:pt.meshmatcap_vert,fragmentShader:pt.meshmatcap_frag},points:{uniforms:In([Ue.points,Ue.fog]),vertexShader:pt.points_vert,fragmentShader:pt.points_frag},dashed:{uniforms:In([Ue.common,Ue.fog,{scale:{value:1},dashSize:{value:1},totalSize:{value:2}}]),vertexShader:pt.linedashed_vert,fragmentShader:pt.linedashed_frag},depth:{uniforms:In([Ue.common,Ue.displacementmap]),vertexShader:pt.depth_vert,fragmentShader:pt.depth_frag},normal:{uniforms:In([Ue.common,Ue.bumpmap,Ue.normalmap,Ue.displacementmap,{opacity:{value:1}}]),vertexShader:pt.meshnormal_vert,fragmentShader:pt.meshnormal_frag},sprite:{uniforms:In([Ue.sprite,Ue.fog]),vertexShader:pt.sprite_vert,fragmentShader:pt.sprite_frag},background:{uniforms:{uvTransform:{value:new ht},t2D:{value:null},backgroundIntensity:{value:1}},vertexShader:pt.background_vert,fragmentShader:pt.background_frag},backgroundCube:{uniforms:{envMap:{value:null},flipEnvMap:{value:-1},backgroundBlurriness:{value:0},backgroundIntensity:{value:1},backgroundRotation:{value:new ht}},vertexShader:pt.backgroundCube_vert,fragmentShader:pt.backgroundCube_frag},cube:{uniforms:{tCube:{value:null},tFlip:{value:-1},opacity:{value:1}},vertexShader:pt.cube_vert,fragmentShader:pt.cube_frag},equirect:{uniforms:{tEquirect:{value:null}},vertexShader:pt.equirect_vert,fragmentShader:pt.equirect_frag},distance:{uniforms:In([Ue.common,Ue.displacementmap,{referencePosition:{value:new K},nearDistance:{value:1},farDistance:{value:1e3}}]),vertexShader:pt.distance_vert,fragmentShader:pt.distance_frag},shadow:{uniforms:In([Ue.lights,Ue.fog,{color:{value:new At(0)},opacity:{value:1}}]),vertexShader:pt.shadow_vert,fragmentShader:pt.shadow_frag}};Bi.physical={uniforms:In([Bi.standard.uniforms,{clearcoat:{value:0},clearcoatMap:{value:null},clearcoatMapTransform:{value:new ht},clearcoatNormalMap:{value:null},clearcoatNormalMapTransform:{value:new ht},clearcoatNormalScale:{value:new ct(1,1)},clearcoatRoughness:{value:0},clearcoatRoughnessMap:{value:null},clearcoatRoughnessMapTransform:{value:new ht},dispersion:{value:0},iridescence:{value:0},iridescenceMap:{value:null},iridescenceMapTransform:{value:new ht},iridescenceIOR:{value:1.3},iridescenceThicknessMinimum:{value:100},iridescenceThicknessMaximum:{value:400},iridescenceThicknessMap:{value:null},iridescenceThicknessMapTransform:{value:new ht},sheen:{value:0},sheenColor:{value:new At(0)},sheenColorMap:{value:null},sheenColorMapTransform:{value:new ht},sheenRoughness:{value:1},sheenRoughnessMap:{value:null},sheenRoughnessMapTransform:{value:new ht},transmission:{value:0},transmissionMap:{value:null},transmissionMapTransform:{value:new ht},transmissionSamplerSize:{value:new ct},transmissionSamplerMap:{value:null},thickness:{value:0},thicknessMap:{value:null},thicknessMapTransform:{value:new ht},attenuationDistance:{value:0},attenuationColor:{value:new At(0)},specularColor:{value:new At(1,1,1)},specularColorMap:{value:null},specularColorMapTransform:{value:new ht},specularIntensity:{value:1},specularIntensityMap:{value:null},specularIntensityMapTransform:{value:new ht},anisotropyVector:{value:new ct},anisotropyMap:{value:null},anisotropyMapTransform:{value:new ht}}]),vertexShader:pt.meshphysical_vert,fragmentShader:pt.meshphysical_frag};const Ic={r:0,b:0,g:0},Cs=new ji,iT=new Jt;function aT(o,e,i,s,l,c){const d=new At(0);let p=l===!0?0:1,m,h,v=null,y=0,g=null;function x(C){let U=C.isScene===!0?C.background:null;if(U&&U.isTexture){const N=C.backgroundBlurriness>0;U=e.get(U,N)}return U}function E(C){let U=!1;const N=x(C);N===null?b(d,p):N&&N.isColor&&(b(N,1),U=!0);const V=o.xr.getEnvironmentBlendMode();V==="additive"?i.buffers.color.setClear(0,0,0,1,c):V==="alpha-blend"&&i.buffers.color.setClear(0,0,0,0,c),(o.autoClear||U)&&(i.buffers.depth.setTest(!0),i.buffers.depth.setMask(!0),i.buffers.color.setMask(!0),o.clear(o.autoClearColor,o.autoClearDepth,o.autoClearStencil))}function w(C,U){const N=x(U);N&&(N.isCubeTexture||N.mapping===Jc)?(h===void 0&&(h=new Wi(new $o(1,1,1),new qi({name:"BackgroundCubeMaterial",uniforms:kr(Bi.backgroundCube.uniforms),vertexShader:Bi.backgroundCube.vertexShader,fragmentShader:Bi.backgroundCube.fragmentShader,side:qn,depthTest:!1,depthWrite:!1,fog:!1,allowOverride:!1})),h.geometry.deleteAttribute("normal"),h.geometry.deleteAttribute("uv"),h.onBeforeRender=function(V,H,F){this.matrixWorld.copyPosition(F.matrixWorld)},Object.defineProperty(h.material,"envMap",{get:function(){return this.uniforms.envMap.value}}),s.update(h)),Cs.copy(U.backgroundRotation),Cs.x*=-1,Cs.y*=-1,Cs.z*=-1,N.isCubeTexture&&N.isRenderTargetTexture===!1&&(Cs.y*=-1,Cs.z*=-1),h.material.uniforms.envMap.value=N,h.material.uniforms.flipEnvMap.value=N.isCubeTexture&&N.isRenderTargetTexture===!1?-1:1,h.material.uniforms.backgroundBlurriness.value=U.backgroundBlurriness,h.material.uniforms.backgroundIntensity.value=U.backgroundIntensity,h.material.uniforms.backgroundRotation.value.setFromMatrix4(iT.makeRotationFromEuler(Cs)),h.material.toneMapped=Tt.getTransfer(N.colorSpace)!==zt,(v!==N||y!==N.version||g!==o.toneMapping)&&(h.material.needsUpdate=!0,v=N,y=N.version,g=o.toneMapping),h.layers.enableAll(),C.unshift(h,h.geometry,h.material,0,0,null)):N&&N.isTexture&&(m===void 0&&(m=new Wi(new eu(2,2),new qi({name:"BackgroundMaterial",uniforms:kr(Bi.background.uniforms),vertexShader:Bi.background.vertexShader,fragmentShader:Bi.background.fragmentShader,side:ss,depthTest:!1,depthWrite:!1,fog:!1,allowOverride:!1})),m.geometry.deleteAttribute("normal"),Object.defineProperty(m.material,"map",{get:function(){return this.uniforms.t2D.value}}),s.update(m)),m.material.uniforms.t2D.value=N,m.material.uniforms.backgroundIntensity.value=U.backgroundIntensity,m.material.toneMapped=Tt.getTransfer(N.colorSpace)!==zt,N.matrixAutoUpdate===!0&&N.updateMatrix(),m.material.uniforms.uvTransform.value.copy(N.matrix),(v!==N||y!==N.version||g!==o.toneMapping)&&(m.material.needsUpdate=!0,v=N,y=N.version,g=o.toneMapping),m.layers.enableAll(),C.unshift(m,m.geometry,m.material,0,0,null))}function b(C,U){C.getRGB(Ic,Ev(o)),i.buffers.color.setClear(Ic.r,Ic.g,Ic.b,U,c)}function S(){h!==void 0&&(h.geometry.dispose(),h.material.dispose(),h=void 0),m!==void 0&&(m.geometry.dispose(),m.material.dispose(),m=void 0)}return{getClearColor:function(){return d},setClearColor:function(C,U=1){d.set(C),p=U,b(d,p)},getClearAlpha:function(){return p},setClearAlpha:function(C){p=C,b(d,p)},render:E,addToRenderList:w,dispose:S}}function sT(o,e){const i=o.getParameter(o.MAX_VERTEX_ATTRIBS),s={},l=g(null);let c=l,d=!1;function p(G,te,se,ue,ee){let P=!1;const z=y(G,ue,se,te);c!==z&&(c=z,h(c.object)),P=x(G,ue,se,ee),P&&E(G,ue,se,ee),ee!==null&&e.update(ee,o.ELEMENT_ARRAY_BUFFER),(P||d)&&(d=!1,N(G,te,se,ue),ee!==null&&o.bindBuffer(o.ELEMENT_ARRAY_BUFFER,e.get(ee).buffer))}function m(){return o.createVertexArray()}function h(G){return o.bindVertexArray(G)}function v(G){return o.deleteVertexArray(G)}function y(G,te,se,ue){const ee=ue.wireframe===!0;let P=s[te.id];P===void 0&&(P={},s[te.id]=P);const z=G.isInstancedMesh===!0?G.id:0;let ce=P[z];ce===void 0&&(ce={},P[z]=ce);let pe=ce[se.id];pe===void 0&&(pe={},ce[se.id]=pe);let Ee=pe[ee];return Ee===void 0&&(Ee=g(m()),pe[ee]=Ee),Ee}function g(G){const te=[],se=[],ue=[];for(let ee=0;ee<i;ee++)te[ee]=0,se[ee]=0,ue[ee]=0;return{geometry:null,program:null,wireframe:!1,newAttributes:te,enabledAttributes:se,attributeDivisors:ue,object:G,attributes:{},index:null}}function x(G,te,se,ue){const ee=c.attributes,P=te.attributes;let z=0;const ce=se.getAttributes();for(const pe in ce)if(ce[pe].location>=0){const I=ee[pe];let Y=P[pe];if(Y===void 0&&(pe==="instanceMatrix"&&G.instanceMatrix&&(Y=G.instanceMatrix),pe==="instanceColor"&&G.instanceColor&&(Y=G.instanceColor)),I===void 0||I.attribute!==Y||Y&&I.data!==Y.data)return!0;z++}return c.attributesNum!==z||c.index!==ue}function E(G,te,se,ue){const ee={},P=te.attributes;let z=0;const ce=se.getAttributes();for(const pe in ce)if(ce[pe].location>=0){let I=P[pe];I===void 0&&(pe==="instanceMatrix"&&G.instanceMatrix&&(I=G.instanceMatrix),pe==="instanceColor"&&G.instanceColor&&(I=G.instanceColor));const Y={};Y.attribute=I,I&&I.data&&(Y.data=I.data),ee[pe]=Y,z++}c.attributes=ee,c.attributesNum=z,c.index=ue}function w(){const G=c.newAttributes;for(let te=0,se=G.length;te<se;te++)G[te]=0}function b(G){S(G,0)}function S(G,te){const se=c.newAttributes,ue=c.enabledAttributes,ee=c.attributeDivisors;se[G]=1,ue[G]===0&&(o.enableVertexAttribArray(G),ue[G]=1),ee[G]!==te&&(o.vertexAttribDivisor(G,te),ee[G]=te)}function C(){const G=c.newAttributes,te=c.enabledAttributes;for(let se=0,ue=te.length;se<ue;se++)te[se]!==G[se]&&(o.disableVertexAttribArray(se),te[se]=0)}function U(G,te,se,ue,ee,P,z){z===!0?o.vertexAttribIPointer(G,te,se,ee,P):o.vertexAttribPointer(G,te,se,ue,ee,P)}function N(G,te,se,ue){w();const ee=ue.attributes,P=se.getAttributes(),z=te.defaultAttributeValues;for(const ce in P){const pe=P[ce];if(pe.location>=0){let Ee=ee[ce];if(Ee===void 0&&(ce==="instanceMatrix"&&G.instanceMatrix&&(Ee=G.instanceMatrix),ce==="instanceColor"&&G.instanceColor&&(Ee=G.instanceColor)),Ee!==void 0){const I=Ee.normalized,Y=Ee.itemSize,ve=e.get(Ee);if(ve===void 0)continue;const Re=ve.buffer,Fe=ve.type,ie=ve.bytesPerElement,xe=Fe===o.INT||Fe===o.UNSIGNED_INT||Ee.gpuType===Xh;if(Ee.isInterleavedBufferAttribute){const Te=Ee.data,ke=Te.stride,Ke=Ee.offset;if(Te.isInstancedInterleavedBuffer){for(let $e=0;$e<pe.locationSize;$e++)S(pe.location+$e,Te.meshPerAttribute);G.isInstancedMesh!==!0&&ue._maxInstanceCount===void 0&&(ue._maxInstanceCount=Te.meshPerAttribute*Te.count)}else for(let $e=0;$e<pe.locationSize;$e++)b(pe.location+$e);o.bindBuffer(o.ARRAY_BUFFER,Re);for(let $e=0;$e<pe.locationSize;$e++)U(pe.location+$e,Y/pe.locationSize,Fe,I,ke*ie,(Ke+Y/pe.locationSize*$e)*ie,xe)}else{if(Ee.isInstancedBufferAttribute){for(let Te=0;Te<pe.locationSize;Te++)S(pe.location+Te,Ee.meshPerAttribute);G.isInstancedMesh!==!0&&ue._maxInstanceCount===void 0&&(ue._maxInstanceCount=Ee.meshPerAttribute*Ee.count)}else for(let Te=0;Te<pe.locationSize;Te++)b(pe.location+Te);o.bindBuffer(o.ARRAY_BUFFER,Re);for(let Te=0;Te<pe.locationSize;Te++)U(pe.location+Te,Y/pe.locationSize,Fe,I,Y*ie,Y/pe.locationSize*Te*ie,xe)}}else if(z!==void 0){const I=z[ce];if(I!==void 0)switch(I.length){case 2:o.vertexAttrib2fv(pe.location,I);break;case 3:o.vertexAttrib3fv(pe.location,I);break;case 4:o.vertexAttrib4fv(pe.location,I);break;default:o.vertexAttrib1fv(pe.location,I)}}}}C()}function V(){D();for(const G in s){const te=s[G];for(const se in te){const ue=te[se];for(const ee in ue){const P=ue[ee];for(const z in P)v(P[z].object),delete P[z];delete ue[ee]}}delete s[G]}}function H(G){if(s[G.id]===void 0)return;const te=s[G.id];for(const se in te){const ue=te[se];for(const ee in ue){const P=ue[ee];for(const z in P)v(P[z].object),delete P[z];delete ue[ee]}}delete s[G.id]}function F(G){for(const te in s){const se=s[te];for(const ue in se){const ee=se[ue];if(ee[G.id]===void 0)continue;const P=ee[G.id];for(const z in P)v(P[z].object),delete P[z];delete ee[G.id]}}}function T(G){for(const te in s){const se=s[te],ue=G.isInstancedMesh===!0?G.id:0,ee=se[ue];if(ee!==void 0){for(const P in ee){const z=ee[P];for(const ce in z)v(z[ce].object),delete z[ce];delete ee[P]}delete se[ue],Object.keys(se).length===0&&delete s[te]}}}function D(){le(),d=!0,c!==l&&(c=l,h(c.object))}function le(){l.geometry=null,l.program=null,l.wireframe=!1}return{setup:p,reset:D,resetDefaultState:le,dispose:V,releaseStatesOfGeometry:H,releaseStatesOfObject:T,releaseStatesOfProgram:F,initAttributes:w,enableAttribute:b,disableUnusedAttributes:C}}function rT(o,e,i){let s;function l(h){s=h}function c(h,v){o.drawArrays(s,h,v),i.update(v,s,1)}function d(h,v,y){y!==0&&(o.drawArraysInstanced(s,h,v,y),i.update(v,s,y))}function p(h,v,y){if(y===0)return;e.get("WEBGL_multi_draw").multiDrawArraysWEBGL(s,h,0,v,0,y);let x=0;for(let E=0;E<y;E++)x+=v[E];i.update(x,s,1)}function m(h,v,y,g){if(y===0)return;const x=e.get("WEBGL_multi_draw");if(x===null)for(let E=0;E<h.length;E++)d(h[E],v[E],g[E]);else{x.multiDrawArraysInstancedWEBGL(s,h,0,v,0,g,0,y);let E=0;for(let w=0;w<y;w++)E+=v[w]*g[w];i.update(E,s,1)}}this.setMode=l,this.render=c,this.renderInstances=d,this.renderMultiDraw=p,this.renderMultiDrawInstances=m}function oT(o,e,i,s){let l;function c(){if(l!==void 0)return l;if(e.has("EXT_texture_filter_anisotropic")===!0){const F=e.get("EXT_texture_filter_anisotropic");l=o.getParameter(F.MAX_TEXTURE_MAX_ANISOTROPY_EXT)}else l=0;return l}function d(F){return!(F!==Di&&s.convert(F)!==o.getParameter(o.IMPLEMENTATION_COLOR_READ_FORMAT))}function p(F){const T=F===ba&&(e.has("EXT_color_buffer_half_float")||e.has("EXT_color_buffer_float"));return!(F!==ri&&s.convert(F)!==o.getParameter(o.IMPLEMENTATION_COLOR_READ_TYPE)&&F!==Hi&&!T)}function m(F){if(F==="highp"){if(o.getShaderPrecisionFormat(o.VERTEX_SHADER,o.HIGH_FLOAT).precision>0&&o.getShaderPrecisionFormat(o.FRAGMENT_SHADER,o.HIGH_FLOAT).precision>0)return"highp";F="mediump"}return F==="mediump"&&o.getShaderPrecisionFormat(o.VERTEX_SHADER,o.MEDIUM_FLOAT).precision>0&&o.getShaderPrecisionFormat(o.FRAGMENT_SHADER,o.MEDIUM_FLOAT).precision>0?"mediump":"lowp"}let h=i.precision!==void 0?i.precision:"highp";const v=m(h);v!==h&&(at("WebGLRenderer:",h,"not supported, using",v,"instead."),h=v);const y=i.logarithmicDepthBuffer===!0,g=i.reversedDepthBuffer===!0&&e.has("EXT_clip_control"),x=o.getParameter(o.MAX_TEXTURE_IMAGE_UNITS),E=o.getParameter(o.MAX_VERTEX_TEXTURE_IMAGE_UNITS),w=o.getParameter(o.MAX_TEXTURE_SIZE),b=o.getParameter(o.MAX_CUBE_MAP_TEXTURE_SIZE),S=o.getParameter(o.MAX_VERTEX_ATTRIBS),C=o.getParameter(o.MAX_VERTEX_UNIFORM_VECTORS),U=o.getParameter(o.MAX_VARYING_VECTORS),N=o.getParameter(o.MAX_FRAGMENT_UNIFORM_VECTORS),V=o.getParameter(o.MAX_SAMPLES),H=o.getParameter(o.SAMPLES);return{isWebGL2:!0,getMaxAnisotropy:c,getMaxPrecision:m,textureFormatReadable:d,textureTypeReadable:p,precision:h,logarithmicDepthBuffer:y,reversedDepthBuffer:g,maxTextures:x,maxVertexTextures:E,maxTextureSize:w,maxCubemapSize:b,maxAttributes:S,maxVertexUniforms:C,maxVaryings:U,maxFragmentUniforms:N,maxSamples:V,samples:H}}function lT(o){const e=this;let i=null,s=0,l=!1,c=!1;const d=new ns,p=new ht,m={value:null,needsUpdate:!1};this.uniform=m,this.numPlanes=0,this.numIntersection=0,this.init=function(y,g){const x=y.length!==0||g||s!==0||l;return l=g,s=y.length,x},this.beginShadows=function(){c=!0,v(null)},this.endShadows=function(){c=!1},this.setGlobalState=function(y,g){i=v(y,g,0)},this.setState=function(y,g,x){const E=y.clippingPlanes,w=y.clipIntersection,b=y.clipShadows,S=o.get(y);if(!l||E===null||E.length===0||c&&!b)c?v(null):h();else{const C=c?0:s,U=C*4;let N=S.clippingState||null;m.value=N,N=v(E,g,U,x);for(let V=0;V!==U;++V)N[V]=i[V];S.clippingState=N,this.numIntersection=w?this.numPlanes:0,this.numPlanes+=C}};function h(){m.value!==i&&(m.value=i,m.needsUpdate=s>0),e.numPlanes=s,e.numIntersection=0}function v(y,g,x,E){const w=y!==null?y.length:0;let b=null;if(w!==0){if(b=m.value,E!==!0||b===null){const S=x+w*4,C=g.matrixWorldInverse;p.getNormalMatrix(C),(b===null||b.length<S)&&(b=new Float32Array(S));for(let U=0,N=x;U!==w;++U,N+=4)d.copy(y[U]).applyMatrix4(C,p),d.normal.toArray(b,N),b[N+3]=d.constant}m.value=b,m.needsUpdate=!0}return e.numPlanes=w,e.numIntersection=0,b}}const as=4,b_=[.125,.215,.35,.446,.526,.582],Ls=20,cT=256,ko=new Rv,M_=new At;let Hd=null,Gd=0,Vd=0,kd=!1;const uT=new K;class E_{constructor(e){this._renderer=e,this._pingPongRenderTarget=null,this._lodMax=0,this._cubeSize=0,this._sizeLods=[],this._sigmas=[],this._lodMeshes=[],this._backgroundBox=null,this._cubemapMaterial=null,this._equirectMaterial=null,this._blurMaterial=null,this._ggxMaterial=null}fromScene(e,i=0,s=.1,l=100,c={}){const{size:d=256,position:p=uT}=c;Hd=this._renderer.getRenderTarget(),Gd=this._renderer.getActiveCubeFace(),Vd=this._renderer.getActiveMipmapLevel(),kd=this._renderer.xr.enabled,this._renderer.xr.enabled=!1,this._setSize(d);const m=this._allocateTargets();return m.depthBuffer=!0,this._sceneToCubeUV(e,s,l,m,p),i>0&&this._blur(m,0,0,i),this._applyPMREM(m),this._cleanup(m),m}fromEquirectangular(e,i=null){return this._fromTexture(e,i)}fromCubemap(e,i=null){return this._fromTexture(e,i)}compileCubemapShader(){this._cubemapMaterial===null&&(this._cubemapMaterial=R_(),this._compileMaterial(this._cubemapMaterial))}compileEquirectangularShader(){this._equirectMaterial===null&&(this._equirectMaterial=A_(),this._compileMaterial(this._equirectMaterial))}dispose(){this._dispose(),this._cubemapMaterial!==null&&this._cubemapMaterial.dispose(),this._equirectMaterial!==null&&this._equirectMaterial.dispose(),this._backgroundBox!==null&&(this._backgroundBox.geometry.dispose(),this._backgroundBox.material.dispose())}_setSize(e){this._lodMax=Math.floor(Math.log2(e)),this._cubeSize=Math.pow(2,this._lodMax)}_dispose(){this._blurMaterial!==null&&this._blurMaterial.dispose(),this._ggxMaterial!==null&&this._ggxMaterial.dispose(),this._pingPongRenderTarget!==null&&this._pingPongRenderTarget.dispose();for(let e=0;e<this._lodMeshes.length;e++)this._lodMeshes[e].geometry.dispose()}_cleanup(e){this._renderer.setRenderTarget(Hd,Gd,Vd),this._renderer.xr.enabled=kd,e.scissorTest=!1,Lr(e,0,0,e.width,e.height)}_fromTexture(e,i){e.mapping===Is||e.mapping===Hr?this._setSize(e.image.length===0?16:e.image[0].width||e.image[0].image.width):this._setSize(e.image.width/4),Hd=this._renderer.getRenderTarget(),Gd=this._renderer.getActiveCubeFace(),Vd=this._renderer.getActiveMipmapLevel(),kd=this._renderer.xr.enabled,this._renderer.xr.enabled=!1;const s=i||this._allocateTargets();return this._textureToCubeUV(e,s),this._applyPMREM(s),this._cleanup(s),s}_allocateTargets(){const e=3*Math.max(this._cubeSize,112),i=4*this._cubeSize,s={magFilter:Dn,minFilter:Dn,generateMipmaps:!1,type:ba,format:Di,colorSpace:Vr,depthBuffer:!1},l=T_(e,i,s);if(this._pingPongRenderTarget===null||this._pingPongRenderTarget.width!==e||this._pingPongRenderTarget.height!==i){this._pingPongRenderTarget!==null&&this._dispose(),this._pingPongRenderTarget=T_(e,i,s);const{_lodMax:c}=this;({lodMeshes:this._lodMeshes,sizeLods:this._sizeLods,sigmas:this._sigmas}=fT(c)),this._blurMaterial=hT(c,e,i),this._ggxMaterial=dT(c,e,i)}return l}_compileMaterial(e){const i=new Wi(new vi,e);this._renderer.compile(i,ko)}_sceneToCubeUV(e,i,s,l,c){const m=new si(90,1,i,s),h=[1,-1,1,1,1,1],v=[1,1,1,-1,-1,-1],y=this._renderer,g=y.autoClear,x=y.toneMapping;y.getClearColor(M_),y.toneMapping=Vi,y.autoClear=!1,y.state.buffers.depth.getReversed()&&(y.setRenderTarget(l),y.clearDepth(),y.setRenderTarget(null)),this._backgroundBox===null&&(this._backgroundBox=new Wi(new $o,new yv({name:"PMREM.Background",side:qn,depthWrite:!1,depthTest:!1})));const w=this._backgroundBox,b=w.material;let S=!1;const C=e.background;C?C.isColor&&(b.color.copy(C),e.background=null,S=!0):(b.color.copy(M_),S=!0);for(let U=0;U<6;U++){const N=U%3;N===0?(m.up.set(0,h[U],0),m.position.set(c.x,c.y,c.z),m.lookAt(c.x+v[U],c.y,c.z)):N===1?(m.up.set(0,0,h[U]),m.position.set(c.x,c.y,c.z),m.lookAt(c.x,c.y+v[U],c.z)):(m.up.set(0,h[U],0),m.position.set(c.x,c.y,c.z),m.lookAt(c.x,c.y,c.z+v[U]));const V=this._cubeSize;Lr(l,N*V,U>2?V:0,V,V),y.setRenderTarget(l),S&&y.render(w,m),y.render(e,m)}y.toneMapping=x,y.autoClear=g,e.background=C}_textureToCubeUV(e,i){const s=this._renderer,l=e.mapping===Is||e.mapping===Hr;l?(this._cubemapMaterial===null&&(this._cubemapMaterial=R_()),this._cubemapMaterial.uniforms.flipEnvMap.value=e.isRenderTargetTexture===!1?-1:1):this._equirectMaterial===null&&(this._equirectMaterial=A_());const c=l?this._cubemapMaterial:this._equirectMaterial,d=this._lodMeshes[0];d.material=c;const p=c.uniforms;p.envMap.value=e;const m=this._cubeSize;Lr(i,0,0,3*m,2*m),s.setRenderTarget(i),s.render(d,ko)}_applyPMREM(e){const i=this._renderer,s=i.autoClear;i.autoClear=!1;const l=this._lodMeshes.length;for(let c=1;c<l;c++)this._applyGGXFilter(e,c-1,c);i.autoClear=s}_applyGGXFilter(e,i,s){const l=this._renderer,c=this._pingPongRenderTarget,d=this._ggxMaterial,p=this._lodMeshes[s];p.material=d;const m=d.uniforms,h=s/(this._lodMeshes.length-1),v=i/(this._lodMeshes.length-1),y=Math.sqrt(h*h-v*v),g=0+h*1.25,x=y*g,{_lodMax:E}=this,w=this._sizeLods[s],b=3*w*(s>E-as?s-E+as:0),S=4*(this._cubeSize-w);m.envMap.value=e.texture,m.roughness.value=x,m.mipInt.value=E-i,Lr(c,b,S,3*w,2*w),l.setRenderTarget(c),l.render(p,ko),m.envMap.value=c.texture,m.roughness.value=0,m.mipInt.value=E-s,Lr(e,b,S,3*w,2*w),l.setRenderTarget(e),l.render(p,ko)}_blur(e,i,s,l,c){const d=this._pingPongRenderTarget;this._halfBlur(e,d,i,s,l,"latitudinal",c),this._halfBlur(d,e,s,s,l,"longitudinal",c)}_halfBlur(e,i,s,l,c,d,p){const m=this._renderer,h=this._blurMaterial;d!=="latitudinal"&&d!=="longitudinal"&&Dt("blur direction must be either latitudinal or longitudinal!");const v=3,y=this._lodMeshes[l];y.material=h;const g=h.uniforms,x=this._sizeLods[s]-1,E=isFinite(c)?Math.PI/(2*x):2*Math.PI/(2*Ls-1),w=c/E,b=isFinite(c)?1+Math.floor(v*w):Ls;b>Ls&&at(`sigmaRadians, ${c}, is too large and will clip, as it requested ${b} samples when the maximum is set to ${Ls}`);const S=[];let C=0;for(let F=0;F<Ls;++F){const T=F/w,D=Math.exp(-T*T/2);S.push(D),F===0?C+=D:F<b&&(C+=2*D)}for(let F=0;F<S.length;F++)S[F]=S[F]/C;g.envMap.value=e.texture,g.samples.value=b,g.weights.value=S,g.latitudinal.value=d==="latitudinal",p&&(g.poleAxis.value=p);const{_lodMax:U}=this;g.dTheta.value=E,g.mipInt.value=U-s;const N=this._sizeLods[l],V=3*N*(l>U-as?l-U+as:0),H=4*(this._cubeSize-N);Lr(i,V,H,3*N,2*N),m.setRenderTarget(i),m.render(y,ko)}}function fT(o){const e=[],i=[],s=[];let l=o;const c=o-as+1+b_.length;for(let d=0;d<c;d++){const p=Math.pow(2,l);e.push(p);let m=1/p;d>o-as?m=b_[d-o+as-1]:d===0&&(m=0),i.push(m);const h=1/(p-2),v=-h,y=1+h,g=[v,v,y,v,y,y,v,v,y,y,v,y],x=6,E=6,w=3,b=2,S=1,C=new Float32Array(w*E*x),U=new Float32Array(b*E*x),N=new Float32Array(S*E*x);for(let H=0;H<x;H++){const F=H%3*2/3-1,T=H>2?0:-1,D=[F,T,0,F+2/3,T,0,F+2/3,T+1,0,F,T,0,F+2/3,T+1,0,F,T+1,0];C.set(D,w*E*H),U.set(g,b*E*H);const le=[H,H,H,H,H,H];N.set(le,S*E*H)}const V=new vi;V.setAttribute("position",new Ni(C,w)),V.setAttribute("uv",new Ni(U,b)),V.setAttribute("faceIndex",new Ni(N,S)),s.push(new Wi(V,null)),l>as&&l--}return{lodMeshes:s,sizeLods:e,sigmas:i}}function T_(o,e,i){const s=new ki(o,e,i);return s.texture.mapping=Jc,s.texture.name="PMREM.cubeUv",s.scissorTest=!0,s}function Lr(o,e,i,s,l){o.viewport.set(e,i,s,l),o.scissor.set(e,i,s,l)}function dT(o,e,i){return new qi({name:"PMREMGGXConvolution",defines:{GGX_SAMPLES:cT,CUBEUV_TEXEL_WIDTH:1/e,CUBEUV_TEXEL_HEIGHT:1/i,CUBEUV_MAX_MIP:`${o}.0`},uniforms:{envMap:{value:null},roughness:{value:0},mipInt:{value:0}},vertexShader:tu(),fragmentShader:`

			precision highp float;
			precision highp int;

			varying vec3 vOutputDirection;

			uniform sampler2D envMap;
			uniform float roughness;
			uniform float mipInt;

			#define ENVMAP_TYPE_CUBE_UV
			#include <cube_uv_reflection_fragment>

			#define PI 3.14159265359

			// Van der Corput radical inverse
			float radicalInverse_VdC(uint bits) {
				bits = (bits << 16u) | (bits >> 16u);
				bits = ((bits & 0x55555555u) << 1u) | ((bits & 0xAAAAAAAAu) >> 1u);
				bits = ((bits & 0x33333333u) << 2u) | ((bits & 0xCCCCCCCCu) >> 2u);
				bits = ((bits & 0x0F0F0F0Fu) << 4u) | ((bits & 0xF0F0F0F0u) >> 4u);
				bits = ((bits & 0x00FF00FFu) << 8u) | ((bits & 0xFF00FF00u) >> 8u);
				return float(bits) * 2.3283064365386963e-10; // / 0x100000000
			}

			// Hammersley sequence
			vec2 hammersley(uint i, uint N) {
				return vec2(float(i) / float(N), radicalInverse_VdC(i));
			}

			// GGX VNDF importance sampling (Eric Heitz 2018)
			// "Sampling the GGX Distribution of Visible Normals"
			// https://jcgt.org/published/0007/04/01/
			vec3 importanceSampleGGX_VNDF(vec2 Xi, vec3 V, float roughness) {
				float alpha = roughness * roughness;

				// Section 4.1: Orthonormal basis
				vec3 T1 = vec3(1.0, 0.0, 0.0);
				vec3 T2 = cross(V, T1);

				// Section 4.2: Parameterization of projected area
				float r = sqrt(Xi.x);
				float phi = 2.0 * PI * Xi.y;
				float t1 = r * cos(phi);
				float t2 = r * sin(phi);
				float s = 0.5 * (1.0 + V.z);
				t2 = (1.0 - s) * sqrt(1.0 - t1 * t1) + s * t2;

				// Section 4.3: Reprojection onto hemisphere
				vec3 Nh = t1 * T1 + t2 * T2 + sqrt(max(0.0, 1.0 - t1 * t1 - t2 * t2)) * V;

				// Section 3.4: Transform back to ellipsoid configuration
				return normalize(vec3(alpha * Nh.x, alpha * Nh.y, max(0.0, Nh.z)));
			}

			void main() {
				vec3 N = normalize(vOutputDirection);
				vec3 V = N; // Assume view direction equals normal for pre-filtering

				vec3 prefilteredColor = vec3(0.0);
				float totalWeight = 0.0;

				// For very low roughness, just sample the environment directly
				if (roughness < 0.001) {
					gl_FragColor = vec4(bilinearCubeUV(envMap, N, mipInt), 1.0);
					return;
				}

				// Tangent space basis for VNDF sampling
				vec3 up = abs(N.z) < 0.999 ? vec3(0.0, 0.0, 1.0) : vec3(1.0, 0.0, 0.0);
				vec3 tangent = normalize(cross(up, N));
				vec3 bitangent = cross(N, tangent);

				for(uint i = 0u; i < uint(GGX_SAMPLES); i++) {
					vec2 Xi = hammersley(i, uint(GGX_SAMPLES));

					// For PMREM, V = N, so in tangent space V is always (0, 0, 1)
					vec3 H_tangent = importanceSampleGGX_VNDF(Xi, vec3(0.0, 0.0, 1.0), roughness);

					// Transform H back to world space
					vec3 H = normalize(tangent * H_tangent.x + bitangent * H_tangent.y + N * H_tangent.z);
					vec3 L = normalize(2.0 * dot(V, H) * H - V);

					float NdotL = max(dot(N, L), 0.0);

					if(NdotL > 0.0) {
						// Sample environment at fixed mip level
						// VNDF importance sampling handles the distribution filtering
						vec3 sampleColor = bilinearCubeUV(envMap, L, mipInt);

						// Weight by NdotL for the split-sum approximation
						// VNDF PDF naturally accounts for the visible microfacet distribution
						prefilteredColor += sampleColor * NdotL;
						totalWeight += NdotL;
					}
				}

				if (totalWeight > 0.0) {
					prefilteredColor = prefilteredColor / totalWeight;
				}

				gl_FragColor = vec4(prefilteredColor, 1.0);
			}
		`,blending:ya,depthTest:!1,depthWrite:!1})}function hT(o,e,i){const s=new Float32Array(Ls),l=new K(0,1,0);return new qi({name:"SphericalGaussianBlur",defines:{n:Ls,CUBEUV_TEXEL_WIDTH:1/e,CUBEUV_TEXEL_HEIGHT:1/i,CUBEUV_MAX_MIP:`${o}.0`},uniforms:{envMap:{value:null},samples:{value:1},weights:{value:s},latitudinal:{value:!1},dTheta:{value:0},mipInt:{value:0},poleAxis:{value:l}},vertexShader:tu(),fragmentShader:`

			precision mediump float;
			precision mediump int;

			varying vec3 vOutputDirection;

			uniform sampler2D envMap;
			uniform int samples;
			uniform float weights[ n ];
			uniform bool latitudinal;
			uniform float dTheta;
			uniform float mipInt;
			uniform vec3 poleAxis;

			#define ENVMAP_TYPE_CUBE_UV
			#include <cube_uv_reflection_fragment>

			vec3 getSample( float theta, vec3 axis ) {

				float cosTheta = cos( theta );
				// Rodrigues' axis-angle rotation
				vec3 sampleDirection = vOutputDirection * cosTheta
					+ cross( axis, vOutputDirection ) * sin( theta )
					+ axis * dot( axis, vOutputDirection ) * ( 1.0 - cosTheta );

				return bilinearCubeUV( envMap, sampleDirection, mipInt );

			}

			void main() {

				vec3 axis = latitudinal ? poleAxis : cross( poleAxis, vOutputDirection );

				if ( all( equal( axis, vec3( 0.0 ) ) ) ) {

					axis = vec3( vOutputDirection.z, 0.0, - vOutputDirection.x );

				}

				axis = normalize( axis );

				gl_FragColor = vec4( 0.0, 0.0, 0.0, 1.0 );
				gl_FragColor.rgb += weights[ 0 ] * getSample( 0.0, axis );

				for ( int i = 1; i < n; i++ ) {

					if ( i >= samples ) {

						break;

					}

					float theta = dTheta * float( i );
					gl_FragColor.rgb += weights[ i ] * getSample( -1.0 * theta, axis );
					gl_FragColor.rgb += weights[ i ] * getSample( theta, axis );

				}

			}
		`,blending:ya,depthTest:!1,depthWrite:!1})}function A_(){return new qi({name:"EquirectangularToCubeUV",uniforms:{envMap:{value:null}},vertexShader:tu(),fragmentShader:`

			precision mediump float;
			precision mediump int;

			varying vec3 vOutputDirection;

			uniform sampler2D envMap;

			#include <common>

			void main() {

				vec3 outputDirection = normalize( vOutputDirection );
				vec2 uv = equirectUv( outputDirection );

				gl_FragColor = vec4( texture2D ( envMap, uv ).rgb, 1.0 );

			}
		`,blending:ya,depthTest:!1,depthWrite:!1})}function R_(){return new qi({name:"CubemapToCubeUV",uniforms:{envMap:{value:null},flipEnvMap:{value:-1}},vertexShader:tu(),fragmentShader:`

			precision mediump float;
			precision mediump int;

			uniform float flipEnvMap;

			varying vec3 vOutputDirection;

			uniform samplerCube envMap;

			void main() {

				gl_FragColor = textureCube( envMap, vec3( flipEnvMap * vOutputDirection.x, vOutputDirection.yz ) );

			}
		`,blending:ya,depthTest:!1,depthWrite:!1})}function tu(){return`

		precision mediump float;
		precision mediump int;

		attribute float faceIndex;

		varying vec3 vOutputDirection;

		// RH coordinate system; PMREM face-indexing convention
		vec3 getDirection( vec2 uv, float face ) {

			uv = 2.0 * uv - 1.0;

			vec3 direction = vec3( uv, 1.0 );

			if ( face == 0.0 ) {

				direction = direction.zyx; // ( 1, v, u ) pos x

			} else if ( face == 1.0 ) {

				direction = direction.xzy;
				direction.xz *= -1.0; // ( -u, 1, -v ) pos y

			} else if ( face == 2.0 ) {

				direction.x *= -1.0; // ( -u, v, 1 ) pos z

			} else if ( face == 3.0 ) {

				direction = direction.zyx;
				direction.xz *= -1.0; // ( -1, v, -u ) neg x

			} else if ( face == 4.0 ) {

				direction = direction.xzy;
				direction.xy *= -1.0; // ( -u, -1, v ) neg y

			} else if ( face == 5.0 ) {

				direction.z *= -1.0; // ( u, v, -1 ) neg z

			}

			return direction;

		}

		void main() {

			vOutputDirection = getDirection( uv, faceIndex );
			gl_Position = vec4( position, 1.0 );

		}
	`}class Cv extends ki{constructor(e=1,i={}){super(e,e,i),this.isWebGLCubeRenderTarget=!0;const s={width:e,height:e,depth:1},l=[s,s,s,s,s,s];this.texture=new bv(l),this._setTextureOptions(i),this.texture.isRenderTargetTexture=!0}fromEquirectangularTexture(e,i){this.texture.type=i.type,this.texture.colorSpace=i.colorSpace,this.texture.generateMipmaps=i.generateMipmaps,this.texture.minFilter=i.minFilter,this.texture.magFilter=i.magFilter;const s={uniforms:{tEquirect:{value:null}},vertexShader:`

				varying vec3 vWorldDirection;

				vec3 transformDirection( in vec3 dir, in mat4 matrix ) {

					return normalize( ( matrix * vec4( dir, 0.0 ) ).xyz );

				}

				void main() {

					vWorldDirection = transformDirection( position, modelMatrix );

					#include <begin_vertex>
					#include <project_vertex>

				}
			`,fragmentShader:`

				uniform sampler2D tEquirect;

				varying vec3 vWorldDirection;

				#include <common>

				void main() {

					vec3 direction = normalize( vWorldDirection );

					vec2 sampleUV = equirectUv( direction );

					gl_FragColor = texture2D( tEquirect, sampleUV );

				}
			`},l=new $o(5,5,5),c=new qi({name:"CubemapFromEquirect",uniforms:kr(s.uniforms),vertexShader:s.vertexShader,fragmentShader:s.fragmentShader,side:qn,blending:ya});c.uniforms.tEquirect.value=i;const d=new Wi(l,c),p=i.minFilter;return i.minFilter===Os&&(i.minFilter=Dn),new xb(1,10,this).update(e,d),i.minFilter=p,d.geometry.dispose(),d.material.dispose(),this}clear(e,i=!0,s=!0,l=!0){const c=e.getRenderTarget();for(let d=0;d<6;d++)e.setRenderTarget(this,d),e.clear(i,s,l);e.setRenderTarget(c)}}function pT(o){let e=new WeakMap,i=new WeakMap,s=null;function l(g,x=!1){return g==null?null:x?d(g):c(g)}function c(g){if(g&&g.isTexture){const x=g.mapping;if(x===hd||x===pd)if(e.has(g)){const E=e.get(g).texture;return p(E,g.mapping)}else{const E=g.image;if(E&&E.height>0){const w=new Cv(E.height);return w.fromEquirectangularTexture(o,g),e.set(g,w),g.addEventListener("dispose",h),p(w.texture,g.mapping)}else return null}}return g}function d(g){if(g&&g.isTexture){const x=g.mapping,E=x===hd||x===pd,w=x===Is||x===Hr;if(E||w){let b=i.get(g);const S=b!==void 0?b.texture.pmremVersion:0;if(g.isRenderTargetTexture&&g.pmremVersion!==S)return s===null&&(s=new E_(o)),b=E?s.fromEquirectangular(g,b):s.fromCubemap(g,b),b.texture.pmremVersion=g.pmremVersion,i.set(g,b),b.texture;if(b!==void 0)return b.texture;{const C=g.image;return E&&C&&C.height>0||w&&C&&m(C)?(s===null&&(s=new E_(o)),b=E?s.fromEquirectangular(g):s.fromCubemap(g),b.texture.pmremVersion=g.pmremVersion,i.set(g,b),g.addEventListener("dispose",v),b.texture):null}}}return g}function p(g,x){return x===hd?g.mapping=Is:x===pd&&(g.mapping=Hr),g}function m(g){let x=0;const E=6;for(let w=0;w<E;w++)g[w]!==void 0&&x++;return x===E}function h(g){const x=g.target;x.removeEventListener("dispose",h);const E=e.get(x);E!==void 0&&(e.delete(x),E.dispose())}function v(g){const x=g.target;x.removeEventListener("dispose",v);const E=i.get(x);E!==void 0&&(i.delete(x),E.dispose())}function y(){e=new WeakMap,i=new WeakMap,s!==null&&(s.dispose(),s=null)}return{get:l,dispose:y}}function mT(o){const e={};function i(s){if(e[s]!==void 0)return e[s];const l=o.getExtension(s);return e[s]=l,l}return{has:function(s){return i(s)!==null},init:function(){i("EXT_color_buffer_float"),i("WEBGL_clip_cull_distance"),i("OES_texture_float_linear"),i("EXT_color_buffer_half_float"),i("WEBGL_multisampled_render_to_texture"),i("WEBGL_render_shared_exponent")},get:function(s){const l=i(s);return l===null&&Kc("WebGLRenderer: "+s+" extension not supported."),l}}}function gT(o,e,i,s){const l={},c=new WeakMap;function d(y){const g=y.target;g.index!==null&&e.remove(g.index);for(const E in g.attributes)e.remove(g.attributes[E]);g.removeEventListener("dispose",d),delete l[g.id];const x=c.get(g);x&&(e.remove(x),c.delete(g)),s.releaseStatesOfGeometry(g),g.isInstancedBufferGeometry===!0&&delete g._maxInstanceCount,i.memory.geometries--}function p(y,g){return l[g.id]===!0||(g.addEventListener("dispose",d),l[g.id]=!0,i.memory.geometries++),g}function m(y){const g=y.attributes;for(const x in g)e.update(g[x],o.ARRAY_BUFFER)}function h(y){const g=[],x=y.index,E=y.attributes.position;let w=0;if(E===void 0)return;if(x!==null){const C=x.array;w=x.version;for(let U=0,N=C.length;U<N;U+=3){const V=C[U+0],H=C[U+1],F=C[U+2];g.push(V,H,H,F,F,V)}}else{const C=E.array;w=E.version;for(let U=0,N=C.length/3-1;U<N;U+=3){const V=U+0,H=U+1,F=U+2;g.push(V,H,H,F,F,V)}}const b=new(E.count>=65535?xv:vv)(g,1);b.version=w;const S=c.get(y);S&&e.remove(S),c.set(y,b)}function v(y){const g=c.get(y);if(g){const x=y.index;x!==null&&g.version<x.version&&h(y)}else h(y);return c.get(y)}return{get:p,update:m,getWireframeAttribute:v}}function _T(o,e,i){let s;function l(g){s=g}let c,d;function p(g){c=g.type,d=g.bytesPerElement}function m(g,x){o.drawElements(s,x,c,g*d),i.update(x,s,1)}function h(g,x,E){E!==0&&(o.drawElementsInstanced(s,x,c,g*d,E),i.update(x,s,E))}function v(g,x,E){if(E===0)return;e.get("WEBGL_multi_draw").multiDrawElementsWEBGL(s,x,0,c,g,0,E);let b=0;for(let S=0;S<E;S++)b+=x[S];i.update(b,s,1)}function y(g,x,E,w){if(E===0)return;const b=e.get("WEBGL_multi_draw");if(b===null)for(let S=0;S<g.length;S++)h(g[S]/d,x[S],w[S]);else{b.multiDrawElementsInstancedWEBGL(s,x,0,c,g,0,w,0,E);let S=0;for(let C=0;C<E;C++)S+=x[C]*w[C];i.update(S,s,1)}}this.setMode=l,this.setIndex=p,this.render=m,this.renderInstances=h,this.renderMultiDraw=v,this.renderMultiDrawInstances=y}function vT(o){const e={geometries:0,textures:0},i={frame:0,calls:0,triangles:0,points:0,lines:0};function s(c,d,p){switch(i.calls++,d){case o.TRIANGLES:i.triangles+=p*(c/3);break;case o.LINES:i.lines+=p*(c/2);break;case o.LINE_STRIP:i.lines+=p*(c-1);break;case o.LINE_LOOP:i.lines+=p*c;break;case o.POINTS:i.points+=p*c;break;default:Dt("WebGLInfo: Unknown draw mode:",d);break}}function l(){i.calls=0,i.triangles=0,i.points=0,i.lines=0}return{memory:e,render:i,programs:null,autoReset:!0,reset:l,update:s}}function xT(o,e,i){const s=new WeakMap,l=new nn;function c(d,p,m){const h=d.morphTargetInfluences,v=p.morphAttributes.position||p.morphAttributes.normal||p.morphAttributes.color,y=v!==void 0?v.length:0;let g=s.get(p);if(g===void 0||g.count!==y){let le=function(){T.dispose(),s.delete(p),p.removeEventListener("dispose",le)};var x=le;g!==void 0&&g.texture.dispose();const E=p.morphAttributes.position!==void 0,w=p.morphAttributes.normal!==void 0,b=p.morphAttributes.color!==void 0,S=p.morphAttributes.position||[],C=p.morphAttributes.normal||[],U=p.morphAttributes.color||[];let N=0;E===!0&&(N=1),w===!0&&(N=2),b===!0&&(N=3);let V=p.attributes.position.count*N,H=1;V>e.maxTextureSize&&(H=Math.ceil(V/e.maxTextureSize),V=e.maxTextureSize);const F=new Float32Array(V*H*4*y),T=new mv(F,V,H,y);T.type=Hi,T.needsUpdate=!0;const D=N*4;for(let G=0;G<y;G++){const te=S[G],se=C[G],ue=U[G],ee=V*H*4*G;for(let P=0;P<te.count;P++){const z=P*D;E===!0&&(l.fromBufferAttribute(te,P),F[ee+z+0]=l.x,F[ee+z+1]=l.y,F[ee+z+2]=l.z,F[ee+z+3]=0),w===!0&&(l.fromBufferAttribute(se,P),F[ee+z+4]=l.x,F[ee+z+5]=l.y,F[ee+z+6]=l.z,F[ee+z+7]=0),b===!0&&(l.fromBufferAttribute(ue,P),F[ee+z+8]=l.x,F[ee+z+9]=l.y,F[ee+z+10]=l.z,F[ee+z+11]=ue.itemSize===4?l.w:1)}}g={count:y,texture:T,size:new ct(V,H)},s.set(p,g),p.addEventListener("dispose",le)}if(d.isInstancedMesh===!0&&d.morphTexture!==null)m.getUniforms().setValue(o,"morphTexture",d.morphTexture,i);else{let E=0;for(let b=0;b<h.length;b++)E+=h[b];const w=p.morphTargetsRelative?1:1-E;m.getUniforms().setValue(o,"morphTargetBaseInfluence",w),m.getUniforms().setValue(o,"morphTargetInfluences",h)}m.getUniforms().setValue(o,"morphTargetsTexture",g.texture,i),m.getUniforms().setValue(o,"morphTargetsTextureSize",g.size)}return{update:c}}function yT(o,e,i,s,l){let c=new WeakMap;function d(h){const v=l.render.frame,y=h.geometry,g=e.get(h,y);if(c.get(g)!==v&&(e.update(g),c.set(g,v)),h.isInstancedMesh&&(h.hasEventListener("dispose",m)===!1&&h.addEventListener("dispose",m),c.get(h)!==v&&(i.update(h.instanceMatrix,o.ARRAY_BUFFER),h.instanceColor!==null&&i.update(h.instanceColor,o.ARRAY_BUFFER),c.set(h,v))),h.isSkinnedMesh){const x=h.skeleton;c.get(x)!==v&&(x.update(),c.set(x,v))}return g}function p(){c=new WeakMap}function m(h){const v=h.target;v.removeEventListener("dispose",m),s.releaseStatesOfObject(v),i.remove(v.instanceMatrix),v.instanceColor!==null&&i.remove(v.instanceColor)}return{update:d,dispose:p}}const ST={[J_]:"LINEAR_TONE_MAPPING",[$_]:"REINHARD_TONE_MAPPING",[ev]:"CINEON_TONE_MAPPING",[tv]:"ACES_FILMIC_TONE_MAPPING",[iv]:"AGX_TONE_MAPPING",[av]:"NEUTRAL_TONE_MAPPING",[nv]:"CUSTOM_TONE_MAPPING"};function bT(o,e,i,s,l){const c=new ki(e,i,{type:o,depthBuffer:s,stencilBuffer:l}),d=new ki(e,i,{type:ba,depthBuffer:!1,stencilBuffer:!1}),p=new vi;p.setAttribute("position",new _i([-1,3,0,-1,-1,0,3,-1,0],3)),p.setAttribute("uv",new _i([0,2,0,0,2,0],2));const m=new fb({uniforms:{tDiffuse:{value:null}},vertexShader:`
			precision highp float;

			uniform mat4 modelViewMatrix;
			uniform mat4 projectionMatrix;

			attribute vec3 position;
			attribute vec2 uv;

			varying vec2 vUv;

			void main() {
				vUv = uv;
				gl_Position = projectionMatrix * modelViewMatrix * vec4( position, 1.0 );
			}`,fragmentShader:`
			precision highp float;

			uniform sampler2D tDiffuse;

			varying vec2 vUv;

			#include <tonemapping_pars_fragment>
			#include <colorspace_pars_fragment>

			void main() {
				gl_FragColor = texture2D( tDiffuse, vUv );

				#ifdef LINEAR_TONE_MAPPING
					gl_FragColor.rgb = LinearToneMapping( gl_FragColor.rgb );
				#elif defined( REINHARD_TONE_MAPPING )
					gl_FragColor.rgb = ReinhardToneMapping( gl_FragColor.rgb );
				#elif defined( CINEON_TONE_MAPPING )
					gl_FragColor.rgb = CineonToneMapping( gl_FragColor.rgb );
				#elif defined( ACES_FILMIC_TONE_MAPPING )
					gl_FragColor.rgb = ACESFilmicToneMapping( gl_FragColor.rgb );
				#elif defined( AGX_TONE_MAPPING )
					gl_FragColor.rgb = AgXToneMapping( gl_FragColor.rgb );
				#elif defined( NEUTRAL_TONE_MAPPING )
					gl_FragColor.rgb = NeutralToneMapping( gl_FragColor.rgb );
				#elif defined( CUSTOM_TONE_MAPPING )
					gl_FragColor.rgb = CustomToneMapping( gl_FragColor.rgb );
				#endif

				#ifdef SRGB_TRANSFER
					gl_FragColor = sRGBTransferOETF( gl_FragColor );
				#endif
			}`,depthTest:!1,depthWrite:!1}),h=new Wi(p,m),v=new Rv(-1,1,1,-1,0,1);let y=null,g=null,x=!1,E,w=null,b=[],S=!1;this.setSize=function(C,U){c.setSize(C,U),d.setSize(C,U);for(let N=0;N<b.length;N++){const V=b[N];V.setSize&&V.setSize(C,U)}},this.setEffects=function(C){b=C,S=b.length>0&&b[0].isRenderPass===!0;const U=c.width,N=c.height;for(let V=0;V<b.length;V++){const H=b[V];H.setSize&&H.setSize(U,N)}},this.begin=function(C,U){if(x||C.toneMapping===Vi&&b.length===0)return!1;if(w=U,U!==null){const N=U.width,V=U.height;(c.width!==N||c.height!==V)&&this.setSize(N,V)}return S===!1&&C.setRenderTarget(c),E=C.toneMapping,C.toneMapping=Vi,!0},this.hasRenderPass=function(){return S},this.end=function(C,U){C.toneMapping=E,x=!0;let N=c,V=d;for(let H=0;H<b.length;H++){const F=b[H];if(F.enabled!==!1&&(F.render(C,V,N,U),F.needsSwap!==!1)){const T=N;N=V,V=T}}if(y!==C.outputColorSpace||g!==C.toneMapping){y=C.outputColorSpace,g=C.toneMapping,m.defines={},Tt.getTransfer(y)===zt&&(m.defines.SRGB_TRANSFER="");const H=ST[g];H&&(m.defines[H]=""),m.needsUpdate=!0}m.uniforms.tDiffuse.value=N.texture,C.setRenderTarget(w),C.render(h,v),w=null,x=!1},this.isCompositing=function(){return x},this.dispose=function(){c.dispose(),d.dispose(),p.dispose(),m.dispose()}}const Dv=new Fn,Bh=new Ko(1,1),Nv=new mv,Uv=new GS,Lv=new bv,w_=[],C_=[],D_=new Float32Array(16),N_=new Float32Array(9),U_=new Float32Array(4);function jr(o,e,i){const s=o[0];if(s<=0||s>0)return o;const l=e*i;let c=w_[l];if(c===void 0&&(c=new Float32Array(l),w_[l]=c),e!==0){s.toArray(c,0);for(let d=1,p=0;d!==e;++d)p+=i,o[d].toArray(c,p)}return c}function vn(o,e){if(o.length!==e.length)return!1;for(let i=0,s=o.length;i<s;i++)if(o[i]!==e[i])return!1;return!0}function xn(o,e){for(let i=0,s=e.length;i<s;i++)o[i]=e[i]}function nu(o,e){let i=C_[e];i===void 0&&(i=new Int32Array(e),C_[e]=i);for(let s=0;s!==e;++s)i[s]=o.allocateTextureUnit();return i}function MT(o,e){const i=this.cache;i[0]!==e&&(o.uniform1f(this.addr,e),i[0]=e)}function ET(o,e){const i=this.cache;if(e.x!==void 0)(i[0]!==e.x||i[1]!==e.y)&&(o.uniform2f(this.addr,e.x,e.y),i[0]=e.x,i[1]=e.y);else{if(vn(i,e))return;o.uniform2fv(this.addr,e),xn(i,e)}}function TT(o,e){const i=this.cache;if(e.x!==void 0)(i[0]!==e.x||i[1]!==e.y||i[2]!==e.z)&&(o.uniform3f(this.addr,e.x,e.y,e.z),i[0]=e.x,i[1]=e.y,i[2]=e.z);else if(e.r!==void 0)(i[0]!==e.r||i[1]!==e.g||i[2]!==e.b)&&(o.uniform3f(this.addr,e.r,e.g,e.b),i[0]=e.r,i[1]=e.g,i[2]=e.b);else{if(vn(i,e))return;o.uniform3fv(this.addr,e),xn(i,e)}}function AT(o,e){const i=this.cache;if(e.x!==void 0)(i[0]!==e.x||i[1]!==e.y||i[2]!==e.z||i[3]!==e.w)&&(o.uniform4f(this.addr,e.x,e.y,e.z,e.w),i[0]=e.x,i[1]=e.y,i[2]=e.z,i[3]=e.w);else{if(vn(i,e))return;o.uniform4fv(this.addr,e),xn(i,e)}}function RT(o,e){const i=this.cache,s=e.elements;if(s===void 0){if(vn(i,e))return;o.uniformMatrix2fv(this.addr,!1,e),xn(i,e)}else{if(vn(i,s))return;U_.set(s),o.uniformMatrix2fv(this.addr,!1,U_),xn(i,s)}}function wT(o,e){const i=this.cache,s=e.elements;if(s===void 0){if(vn(i,e))return;o.uniformMatrix3fv(this.addr,!1,e),xn(i,e)}else{if(vn(i,s))return;N_.set(s),o.uniformMatrix3fv(this.addr,!1,N_),xn(i,s)}}function CT(o,e){const i=this.cache,s=e.elements;if(s===void 0){if(vn(i,e))return;o.uniformMatrix4fv(this.addr,!1,e),xn(i,e)}else{if(vn(i,s))return;D_.set(s),o.uniformMatrix4fv(this.addr,!1,D_),xn(i,s)}}function DT(o,e){const i=this.cache;i[0]!==e&&(o.uniform1i(this.addr,e),i[0]=e)}function NT(o,e){const i=this.cache;if(e.x!==void 0)(i[0]!==e.x||i[1]!==e.y)&&(o.uniform2i(this.addr,e.x,e.y),i[0]=e.x,i[1]=e.y);else{if(vn(i,e))return;o.uniform2iv(this.addr,e),xn(i,e)}}function UT(o,e){const i=this.cache;if(e.x!==void 0)(i[0]!==e.x||i[1]!==e.y||i[2]!==e.z)&&(o.uniform3i(this.addr,e.x,e.y,e.z),i[0]=e.x,i[1]=e.y,i[2]=e.z);else{if(vn(i,e))return;o.uniform3iv(this.addr,e),xn(i,e)}}function LT(o,e){const i=this.cache;if(e.x!==void 0)(i[0]!==e.x||i[1]!==e.y||i[2]!==e.z||i[3]!==e.w)&&(o.uniform4i(this.addr,e.x,e.y,e.z,e.w),i[0]=e.x,i[1]=e.y,i[2]=e.z,i[3]=e.w);else{if(vn(i,e))return;o.uniform4iv(this.addr,e),xn(i,e)}}function OT(o,e){const i=this.cache;i[0]!==e&&(o.uniform1ui(this.addr,e),i[0]=e)}function PT(o,e){const i=this.cache;if(e.x!==void 0)(i[0]!==e.x||i[1]!==e.y)&&(o.uniform2ui(this.addr,e.x,e.y),i[0]=e.x,i[1]=e.y);else{if(vn(i,e))return;o.uniform2uiv(this.addr,e),xn(i,e)}}function IT(o,e){const i=this.cache;if(e.x!==void 0)(i[0]!==e.x||i[1]!==e.y||i[2]!==e.z)&&(o.uniform3ui(this.addr,e.x,e.y,e.z),i[0]=e.x,i[1]=e.y,i[2]=e.z);else{if(vn(i,e))return;o.uniform3uiv(this.addr,e),xn(i,e)}}function FT(o,e){const i=this.cache;if(e.x!==void 0)(i[0]!==e.x||i[1]!==e.y||i[2]!==e.z||i[3]!==e.w)&&(o.uniform4ui(this.addr,e.x,e.y,e.z,e.w),i[0]=e.x,i[1]=e.y,i[2]=e.z,i[3]=e.w);else{if(vn(i,e))return;o.uniform4uiv(this.addr,e),xn(i,e)}}function zT(o,e,i){const s=this.cache,l=i.allocateTextureUnit();s[0]!==l&&(o.uniform1i(this.addr,l),s[0]=l);let c;this.type===o.SAMPLER_2D_SHADOW?(Bh.compareFunction=i.isReversedDepthBuffer()?Qh:Kh,c=Bh):c=Dv,i.setTexture2D(e||c,l)}function BT(o,e,i){const s=this.cache,l=i.allocateTextureUnit();s[0]!==l&&(o.uniform1i(this.addr,l),s[0]=l),i.setTexture3D(e||Uv,l)}function HT(o,e,i){const s=this.cache,l=i.allocateTextureUnit();s[0]!==l&&(o.uniform1i(this.addr,l),s[0]=l),i.setTextureCube(e||Lv,l)}function GT(o,e,i){const s=this.cache,l=i.allocateTextureUnit();s[0]!==l&&(o.uniform1i(this.addr,l),s[0]=l),i.setTexture2DArray(e||Nv,l)}function VT(o){switch(o){case 5126:return MT;case 35664:return ET;case 35665:return TT;case 35666:return AT;case 35674:return RT;case 35675:return wT;case 35676:return CT;case 5124:case 35670:return DT;case 35667:case 35671:return NT;case 35668:case 35672:return UT;case 35669:case 35673:return LT;case 5125:return OT;case 36294:return PT;case 36295:return IT;case 36296:return FT;case 35678:case 36198:case 36298:case 36306:case 35682:return zT;case 35679:case 36299:case 36307:return BT;case 35680:case 36300:case 36308:case 36293:return HT;case 36289:case 36303:case 36311:case 36292:return GT}}function kT(o,e){o.uniform1fv(this.addr,e)}function XT(o,e){const i=jr(e,this.size,2);o.uniform2fv(this.addr,i)}function jT(o,e){const i=jr(e,this.size,3);o.uniform3fv(this.addr,i)}function WT(o,e){const i=jr(e,this.size,4);o.uniform4fv(this.addr,i)}function qT(o,e){const i=jr(e,this.size,4);o.uniformMatrix2fv(this.addr,!1,i)}function YT(o,e){const i=jr(e,this.size,9);o.uniformMatrix3fv(this.addr,!1,i)}function ZT(o,e){const i=jr(e,this.size,16);o.uniformMatrix4fv(this.addr,!1,i)}function KT(o,e){o.uniform1iv(this.addr,e)}function QT(o,e){o.uniform2iv(this.addr,e)}function JT(o,e){o.uniform3iv(this.addr,e)}function $T(o,e){o.uniform4iv(this.addr,e)}function e1(o,e){o.uniform1uiv(this.addr,e)}function t1(o,e){o.uniform2uiv(this.addr,e)}function n1(o,e){o.uniform3uiv(this.addr,e)}function i1(o,e){o.uniform4uiv(this.addr,e)}function a1(o,e,i){const s=this.cache,l=e.length,c=nu(i,l);vn(s,c)||(o.uniform1iv(this.addr,c),xn(s,c));let d;this.type===o.SAMPLER_2D_SHADOW?d=Bh:d=Dv;for(let p=0;p!==l;++p)i.setTexture2D(e[p]||d,c[p])}function s1(o,e,i){const s=this.cache,l=e.length,c=nu(i,l);vn(s,c)||(o.uniform1iv(this.addr,c),xn(s,c));for(let d=0;d!==l;++d)i.setTexture3D(e[d]||Uv,c[d])}function r1(o,e,i){const s=this.cache,l=e.length,c=nu(i,l);vn(s,c)||(o.uniform1iv(this.addr,c),xn(s,c));for(let d=0;d!==l;++d)i.setTextureCube(e[d]||Lv,c[d])}function o1(o,e,i){const s=this.cache,l=e.length,c=nu(i,l);vn(s,c)||(o.uniform1iv(this.addr,c),xn(s,c));for(let d=0;d!==l;++d)i.setTexture2DArray(e[d]||Nv,c[d])}function l1(o){switch(o){case 5126:return kT;case 35664:return XT;case 35665:return jT;case 35666:return WT;case 35674:return qT;case 35675:return YT;case 35676:return ZT;case 5124:case 35670:return KT;case 35667:case 35671:return QT;case 35668:case 35672:return JT;case 35669:case 35673:return $T;case 5125:return e1;case 36294:return t1;case 36295:return n1;case 36296:return i1;case 35678:case 36198:case 36298:case 36306:case 35682:return a1;case 35679:case 36299:case 36307:return s1;case 35680:case 36300:case 36308:case 36293:return r1;case 36289:case 36303:case 36311:case 36292:return o1}}class c1{constructor(e,i,s){this.id=e,this.addr=s,this.cache=[],this.type=i.type,this.setValue=VT(i.type)}}class u1{constructor(e,i,s){this.id=e,this.addr=s,this.cache=[],this.type=i.type,this.size=i.size,this.setValue=l1(i.type)}}class f1{constructor(e){this.id=e,this.seq=[],this.map={}}setValue(e,i,s){const l=this.seq;for(let c=0,d=l.length;c!==d;++c){const p=l[c];p.setValue(e,i[p.id],s)}}}const Xd=/(\w+)(\])?(\[|\.)?/g;function L_(o,e){o.seq.push(e),o.map[e.id]=e}function d1(o,e,i){const s=o.name,l=s.length;for(Xd.lastIndex=0;;){const c=Xd.exec(s),d=Xd.lastIndex;let p=c[1];const m=c[2]==="]",h=c[3];if(m&&(p=p|0),h===void 0||h==="["&&d+2===l){L_(i,h===void 0?new c1(p,o,e):new u1(p,o,e));break}else{let y=i.map[p];y===void 0&&(y=new f1(p),L_(i,y)),i=y}}}class qc{constructor(e,i){this.seq=[],this.map={};const s=e.getProgramParameter(i,e.ACTIVE_UNIFORMS);for(let d=0;d<s;++d){const p=e.getActiveUniform(i,d),m=e.getUniformLocation(i,p.name);d1(p,m,this)}const l=[],c=[];for(const d of this.seq)d.type===e.SAMPLER_2D_SHADOW||d.type===e.SAMPLER_CUBE_SHADOW||d.type===e.SAMPLER_2D_ARRAY_SHADOW?l.push(d):c.push(d);l.length>0&&(this.seq=l.concat(c))}setValue(e,i,s,l){const c=this.map[i];c!==void 0&&c.setValue(e,s,l)}setOptional(e,i,s){const l=i[s];l!==void 0&&this.setValue(e,s,l)}static upload(e,i,s,l){for(let c=0,d=i.length;c!==d;++c){const p=i[c],m=s[p.id];m.needsUpdate!==!1&&p.setValue(e,m.value,l)}}static seqWithValue(e,i){const s=[];for(let l=0,c=e.length;l!==c;++l){const d=e[l];d.id in i&&s.push(d)}return s}}function O_(o,e,i){const s=o.createShader(e);return o.shaderSource(s,i),o.compileShader(s),s}const h1=37297;let p1=0;function m1(o,e){const i=o.split(`
`),s=[],l=Math.max(e-6,0),c=Math.min(e+6,i.length);for(let d=l;d<c;d++){const p=d+1;s.push(`${p===e?">":" "} ${p}: ${i[d]}`)}return s.join(`
`)}const P_=new ht;function g1(o){Tt._getMatrix(P_,Tt.workingColorSpace,o);const e=`mat3( ${P_.elements.map(i=>i.toFixed(4))} )`;switch(Tt.getTransfer(o)){case Yc:return[e,"LinearTransferOETF"];case zt:return[e,"sRGBTransferOETF"];default:return at("WebGLProgram: Unsupported color space: ",o),[e,"LinearTransferOETF"]}}function I_(o,e,i){const s=o.getShaderParameter(e,o.COMPILE_STATUS),c=(o.getShaderInfoLog(e)||"").trim();if(s&&c==="")return"";const d=/ERROR: 0:(\d+)/.exec(c);if(d){const p=parseInt(d[1]);return i.toUpperCase()+`

`+c+`

`+m1(o.getShaderSource(e),p)}else return c}function _1(o,e){const i=g1(e);return[`vec4 ${o}( vec4 value ) {`,`	return ${i[1]}( vec4( value.rgb * ${i[0]}, value.a ) );`,"}"].join(`
`)}const v1={[J_]:"Linear",[$_]:"Reinhard",[ev]:"Cineon",[tv]:"ACESFilmic",[iv]:"AgX",[av]:"Neutral",[nv]:"Custom"};function x1(o,e){const i=v1[e];return i===void 0?(at("WebGLProgram: Unsupported toneMapping:",e),"vec3 "+o+"( vec3 color ) { return LinearToneMapping( color ); }"):"vec3 "+o+"( vec3 color ) { return "+i+"ToneMapping( color ); }"}const Fc=new K;function y1(){Tt.getLuminanceCoefficients(Fc);const o=Fc.x.toFixed(4),e=Fc.y.toFixed(4),i=Fc.z.toFixed(4);return["float luminance( const in vec3 rgb ) {",`	const vec3 weights = vec3( ${o}, ${e}, ${i} );`,"	return dot( weights, rgb );","}"].join(`
`)}function S1(o){return[o.extensionClipCullDistance?"#extension GL_ANGLE_clip_cull_distance : require":"",o.extensionMultiDraw?"#extension GL_ANGLE_multi_draw : require":""].filter(Wo).join(`
`)}function b1(o){const e=[];for(const i in o){const s=o[i];s!==!1&&e.push("#define "+i+" "+s)}return e.join(`
`)}function M1(o,e){const i={},s=o.getProgramParameter(e,o.ACTIVE_ATTRIBUTES);for(let l=0;l<s;l++){const c=o.getActiveAttrib(e,l),d=c.name;let p=1;c.type===o.FLOAT_MAT2&&(p=2),c.type===o.FLOAT_MAT3&&(p=3),c.type===o.FLOAT_MAT4&&(p=4),i[d]={type:c.type,location:o.getAttribLocation(e,d),locationSize:p}}return i}function Wo(o){return o!==""}function F_(o,e){const i=e.numSpotLightShadows+e.numSpotLightMaps-e.numSpotLightShadowsWithMaps;return o.replace(/NUM_DIR_LIGHTS/g,e.numDirLights).replace(/NUM_SPOT_LIGHTS/g,e.numSpotLights).replace(/NUM_SPOT_LIGHT_MAPS/g,e.numSpotLightMaps).replace(/NUM_SPOT_LIGHT_COORDS/g,i).replace(/NUM_RECT_AREA_LIGHTS/g,e.numRectAreaLights).replace(/NUM_POINT_LIGHTS/g,e.numPointLights).replace(/NUM_HEMI_LIGHTS/g,e.numHemiLights).replace(/NUM_DIR_LIGHT_SHADOWS/g,e.numDirLightShadows).replace(/NUM_SPOT_LIGHT_SHADOWS_WITH_MAPS/g,e.numSpotLightShadowsWithMaps).replace(/NUM_SPOT_LIGHT_SHADOWS/g,e.numSpotLightShadows).replace(/NUM_POINT_LIGHT_SHADOWS/g,e.numPointLightShadows)}function z_(o,e){return o.replace(/NUM_CLIPPING_PLANES/g,e.numClippingPlanes).replace(/UNION_CLIPPING_PLANES/g,e.numClippingPlanes-e.numClipIntersection)}const E1=/^[ \t]*#include +<([\w\d./]+)>/gm;function Hh(o){return o.replace(E1,A1)}const T1=new Map;function A1(o,e){let i=pt[e];if(i===void 0){const s=T1.get(e);if(s!==void 0)i=pt[s],at('WebGLRenderer: Shader chunk "%s" has been deprecated. Use "%s" instead.',e,s);else throw new Error("Can not resolve #include <"+e+">")}return Hh(i)}const R1=/#pragma unroll_loop_start\s+for\s*\(\s*int\s+i\s*=\s*(\d+)\s*;\s*i\s*<\s*(\d+)\s*;\s*i\s*\+\+\s*\)\s*{([\s\S]+?)}\s+#pragma unroll_loop_end/g;function B_(o){return o.replace(R1,w1)}function w1(o,e,i,s){let l="";for(let c=parseInt(e);c<parseInt(i);c++)l+=s.replace(/\[\s*i\s*\]/g,"[ "+c+" ]").replace(/UNROLLED_LOOP_INDEX/g,c);return l}function H_(o){let e=`precision ${o.precision} float;
	precision ${o.precision} int;
	precision ${o.precision} sampler2D;
	precision ${o.precision} samplerCube;
	precision ${o.precision} sampler3D;
	precision ${o.precision} sampler2DArray;
	precision ${o.precision} sampler2DShadow;
	precision ${o.precision} samplerCubeShadow;
	precision ${o.precision} sampler2DArrayShadow;
	precision ${o.precision} isampler2D;
	precision ${o.precision} isampler3D;
	precision ${o.precision} isamplerCube;
	precision ${o.precision} isampler2DArray;
	precision ${o.precision} usampler2D;
	precision ${o.precision} usampler3D;
	precision ${o.precision} usamplerCube;
	precision ${o.precision} usampler2DArray;
	`;return o.precision==="highp"?e+=`
#define HIGH_PRECISION`:o.precision==="mediump"?e+=`
#define MEDIUM_PRECISION`:o.precision==="lowp"&&(e+=`
#define LOW_PRECISION`),e}const C1={[Gc]:"SHADOWMAP_TYPE_PCF",[jo]:"SHADOWMAP_TYPE_VSM"};function D1(o){return C1[o.shadowMapType]||"SHADOWMAP_TYPE_BASIC"}const N1={[Is]:"ENVMAP_TYPE_CUBE",[Hr]:"ENVMAP_TYPE_CUBE",[Jc]:"ENVMAP_TYPE_CUBE_UV"};function U1(o){return o.envMap===!1?"ENVMAP_TYPE_CUBE":N1[o.envMapMode]||"ENVMAP_TYPE_CUBE"}const L1={[Hr]:"ENVMAP_MODE_REFRACTION"};function O1(o){return o.envMap===!1?"ENVMAP_MODE_REFLECTION":L1[o.envMapMode]||"ENVMAP_MODE_REFLECTION"}const P1={[Q_]:"ENVMAP_BLENDING_MULTIPLY",[xS]:"ENVMAP_BLENDING_MIX",[yS]:"ENVMAP_BLENDING_ADD"};function I1(o){return o.envMap===!1?"ENVMAP_BLENDING_NONE":P1[o.combine]||"ENVMAP_BLENDING_NONE"}function F1(o){const e=o.envMapCubeUVHeight;if(e===null)return null;const i=Math.log2(e)-2,s=1/e;return{texelWidth:1/(3*Math.max(Math.pow(2,i),112)),texelHeight:s,maxMip:i}}function z1(o,e,i,s){const l=o.getContext(),c=i.defines;let d=i.vertexShader,p=i.fragmentShader;const m=D1(i),h=U1(i),v=O1(i),y=I1(i),g=F1(i),x=S1(i),E=b1(c),w=l.createProgram();let b,S,C=i.glslVersion?"#version "+i.glslVersion+`
`:"";i.isRawShaderMaterial?(b=["#define SHADER_TYPE "+i.shaderType,"#define SHADER_NAME "+i.shaderName,E].filter(Wo).join(`
`),b.length>0&&(b+=`
`),S=["#define SHADER_TYPE "+i.shaderType,"#define SHADER_NAME "+i.shaderName,E].filter(Wo).join(`
`),S.length>0&&(S+=`
`)):(b=[H_(i),"#define SHADER_TYPE "+i.shaderType,"#define SHADER_NAME "+i.shaderName,E,i.extensionClipCullDistance?"#define USE_CLIP_DISTANCE":"",i.batching?"#define USE_BATCHING":"",i.batchingColor?"#define USE_BATCHING_COLOR":"",i.instancing?"#define USE_INSTANCING":"",i.instancingColor?"#define USE_INSTANCING_COLOR":"",i.instancingMorph?"#define USE_INSTANCING_MORPH":"",i.useFog&&i.fog?"#define USE_FOG":"",i.useFog&&i.fogExp2?"#define FOG_EXP2":"",i.map?"#define USE_MAP":"",i.envMap?"#define USE_ENVMAP":"",i.envMap?"#define "+v:"",i.lightMap?"#define USE_LIGHTMAP":"",i.aoMap?"#define USE_AOMAP":"",i.bumpMap?"#define USE_BUMPMAP":"",i.normalMap?"#define USE_NORMALMAP":"",i.normalMapObjectSpace?"#define USE_NORMALMAP_OBJECTSPACE":"",i.normalMapTangentSpace?"#define USE_NORMALMAP_TANGENTSPACE":"",i.displacementMap?"#define USE_DISPLACEMENTMAP":"",i.emissiveMap?"#define USE_EMISSIVEMAP":"",i.anisotropy?"#define USE_ANISOTROPY":"",i.anisotropyMap?"#define USE_ANISOTROPYMAP":"",i.clearcoatMap?"#define USE_CLEARCOATMAP":"",i.clearcoatRoughnessMap?"#define USE_CLEARCOAT_ROUGHNESSMAP":"",i.clearcoatNormalMap?"#define USE_CLEARCOAT_NORMALMAP":"",i.iridescenceMap?"#define USE_IRIDESCENCEMAP":"",i.iridescenceThicknessMap?"#define USE_IRIDESCENCE_THICKNESSMAP":"",i.specularMap?"#define USE_SPECULARMAP":"",i.specularColorMap?"#define USE_SPECULAR_COLORMAP":"",i.specularIntensityMap?"#define USE_SPECULAR_INTENSITYMAP":"",i.roughnessMap?"#define USE_ROUGHNESSMAP":"",i.metalnessMap?"#define USE_METALNESSMAP":"",i.alphaMap?"#define USE_ALPHAMAP":"",i.alphaHash?"#define USE_ALPHAHASH":"",i.transmission?"#define USE_TRANSMISSION":"",i.transmissionMap?"#define USE_TRANSMISSIONMAP":"",i.thicknessMap?"#define USE_THICKNESSMAP":"",i.sheenColorMap?"#define USE_SHEEN_COLORMAP":"",i.sheenRoughnessMap?"#define USE_SHEEN_ROUGHNESSMAP":"",i.mapUv?"#define MAP_UV "+i.mapUv:"",i.alphaMapUv?"#define ALPHAMAP_UV "+i.alphaMapUv:"",i.lightMapUv?"#define LIGHTMAP_UV "+i.lightMapUv:"",i.aoMapUv?"#define AOMAP_UV "+i.aoMapUv:"",i.emissiveMapUv?"#define EMISSIVEMAP_UV "+i.emissiveMapUv:"",i.bumpMapUv?"#define BUMPMAP_UV "+i.bumpMapUv:"",i.normalMapUv?"#define NORMALMAP_UV "+i.normalMapUv:"",i.displacementMapUv?"#define DISPLACEMENTMAP_UV "+i.displacementMapUv:"",i.metalnessMapUv?"#define METALNESSMAP_UV "+i.metalnessMapUv:"",i.roughnessMapUv?"#define ROUGHNESSMAP_UV "+i.roughnessMapUv:"",i.anisotropyMapUv?"#define ANISOTROPYMAP_UV "+i.anisotropyMapUv:"",i.clearcoatMapUv?"#define CLEARCOATMAP_UV "+i.clearcoatMapUv:"",i.clearcoatNormalMapUv?"#define CLEARCOAT_NORMALMAP_UV "+i.clearcoatNormalMapUv:"",i.clearcoatRoughnessMapUv?"#define CLEARCOAT_ROUGHNESSMAP_UV "+i.clearcoatRoughnessMapUv:"",i.iridescenceMapUv?"#define IRIDESCENCEMAP_UV "+i.iridescenceMapUv:"",i.iridescenceThicknessMapUv?"#define IRIDESCENCE_THICKNESSMAP_UV "+i.iridescenceThicknessMapUv:"",i.sheenColorMapUv?"#define SHEEN_COLORMAP_UV "+i.sheenColorMapUv:"",i.sheenRoughnessMapUv?"#define SHEEN_ROUGHNESSMAP_UV "+i.sheenRoughnessMapUv:"",i.specularMapUv?"#define SPECULARMAP_UV "+i.specularMapUv:"",i.specularColorMapUv?"#define SPECULAR_COLORMAP_UV "+i.specularColorMapUv:"",i.specularIntensityMapUv?"#define SPECULAR_INTENSITYMAP_UV "+i.specularIntensityMapUv:"",i.transmissionMapUv?"#define TRANSMISSIONMAP_UV "+i.transmissionMapUv:"",i.thicknessMapUv?"#define THICKNESSMAP_UV "+i.thicknessMapUv:"",i.vertexTangents&&i.flatShading===!1?"#define USE_TANGENT":"",i.vertexColors?"#define USE_COLOR":"",i.vertexAlphas?"#define USE_COLOR_ALPHA":"",i.vertexUv1s?"#define USE_UV1":"",i.vertexUv2s?"#define USE_UV2":"",i.vertexUv3s?"#define USE_UV3":"",i.pointsUvs?"#define USE_POINTS_UV":"",i.flatShading?"#define FLAT_SHADED":"",i.skinning?"#define USE_SKINNING":"",i.morphTargets?"#define USE_MORPHTARGETS":"",i.morphNormals&&i.flatShading===!1?"#define USE_MORPHNORMALS":"",i.morphColors?"#define USE_MORPHCOLORS":"",i.morphTargetsCount>0?"#define MORPHTARGETS_TEXTURE_STRIDE "+i.morphTextureStride:"",i.morphTargetsCount>0?"#define MORPHTARGETS_COUNT "+i.morphTargetsCount:"",i.doubleSided?"#define DOUBLE_SIDED":"",i.flipSided?"#define FLIP_SIDED":"",i.shadowMapEnabled?"#define USE_SHADOWMAP":"",i.shadowMapEnabled?"#define "+m:"",i.sizeAttenuation?"#define USE_SIZEATTENUATION":"",i.numLightProbes>0?"#define USE_LIGHT_PROBES":"",i.logarithmicDepthBuffer?"#define USE_LOGARITHMIC_DEPTH_BUFFER":"",i.reversedDepthBuffer?"#define USE_REVERSED_DEPTH_BUFFER":"","uniform mat4 modelMatrix;","uniform mat4 modelViewMatrix;","uniform mat4 projectionMatrix;","uniform mat4 viewMatrix;","uniform mat3 normalMatrix;","uniform vec3 cameraPosition;","uniform bool isOrthographic;","#ifdef USE_INSTANCING","	attribute mat4 instanceMatrix;","#endif","#ifdef USE_INSTANCING_COLOR","	attribute vec3 instanceColor;","#endif","#ifdef USE_INSTANCING_MORPH","	uniform sampler2D morphTexture;","#endif","attribute vec3 position;","attribute vec3 normal;","attribute vec2 uv;","#ifdef USE_UV1","	attribute vec2 uv1;","#endif","#ifdef USE_UV2","	attribute vec2 uv2;","#endif","#ifdef USE_UV3","	attribute vec2 uv3;","#endif","#ifdef USE_TANGENT","	attribute vec4 tangent;","#endif","#if defined( USE_COLOR_ALPHA )","	attribute vec4 color;","#elif defined( USE_COLOR )","	attribute vec3 color;","#endif","#ifdef USE_SKINNING","	attribute vec4 skinIndex;","	attribute vec4 skinWeight;","#endif",`
`].filter(Wo).join(`
`),S=[H_(i),"#define SHADER_TYPE "+i.shaderType,"#define SHADER_NAME "+i.shaderName,E,i.useFog&&i.fog?"#define USE_FOG":"",i.useFog&&i.fogExp2?"#define FOG_EXP2":"",i.alphaToCoverage?"#define ALPHA_TO_COVERAGE":"",i.map?"#define USE_MAP":"",i.matcap?"#define USE_MATCAP":"",i.envMap?"#define USE_ENVMAP":"",i.envMap?"#define "+h:"",i.envMap?"#define "+v:"",i.envMap?"#define "+y:"",g?"#define CUBEUV_TEXEL_WIDTH "+g.texelWidth:"",g?"#define CUBEUV_TEXEL_HEIGHT "+g.texelHeight:"",g?"#define CUBEUV_MAX_MIP "+g.maxMip+".0":"",i.lightMap?"#define USE_LIGHTMAP":"",i.aoMap?"#define USE_AOMAP":"",i.bumpMap?"#define USE_BUMPMAP":"",i.normalMap?"#define USE_NORMALMAP":"",i.normalMapObjectSpace?"#define USE_NORMALMAP_OBJECTSPACE":"",i.normalMapTangentSpace?"#define USE_NORMALMAP_TANGENTSPACE":"",i.emissiveMap?"#define USE_EMISSIVEMAP":"",i.anisotropy?"#define USE_ANISOTROPY":"",i.anisotropyMap?"#define USE_ANISOTROPYMAP":"",i.clearcoat?"#define USE_CLEARCOAT":"",i.clearcoatMap?"#define USE_CLEARCOATMAP":"",i.clearcoatRoughnessMap?"#define USE_CLEARCOAT_ROUGHNESSMAP":"",i.clearcoatNormalMap?"#define USE_CLEARCOAT_NORMALMAP":"",i.dispersion?"#define USE_DISPERSION":"",i.iridescence?"#define USE_IRIDESCENCE":"",i.iridescenceMap?"#define USE_IRIDESCENCEMAP":"",i.iridescenceThicknessMap?"#define USE_IRIDESCENCE_THICKNESSMAP":"",i.specularMap?"#define USE_SPECULARMAP":"",i.specularColorMap?"#define USE_SPECULAR_COLORMAP":"",i.specularIntensityMap?"#define USE_SPECULAR_INTENSITYMAP":"",i.roughnessMap?"#define USE_ROUGHNESSMAP":"",i.metalnessMap?"#define USE_METALNESSMAP":"",i.alphaMap?"#define USE_ALPHAMAP":"",i.alphaTest?"#define USE_ALPHATEST":"",i.alphaHash?"#define USE_ALPHAHASH":"",i.sheen?"#define USE_SHEEN":"",i.sheenColorMap?"#define USE_SHEEN_COLORMAP":"",i.sheenRoughnessMap?"#define USE_SHEEN_ROUGHNESSMAP":"",i.transmission?"#define USE_TRANSMISSION":"",i.transmissionMap?"#define USE_TRANSMISSIONMAP":"",i.thicknessMap?"#define USE_THICKNESSMAP":"",i.vertexTangents&&i.flatShading===!1?"#define USE_TANGENT":"",i.vertexColors||i.instancingColor?"#define USE_COLOR":"",i.vertexAlphas||i.batchingColor?"#define USE_COLOR_ALPHA":"",i.vertexUv1s?"#define USE_UV1":"",i.vertexUv2s?"#define USE_UV2":"",i.vertexUv3s?"#define USE_UV3":"",i.pointsUvs?"#define USE_POINTS_UV":"",i.gradientMap?"#define USE_GRADIENTMAP":"",i.flatShading?"#define FLAT_SHADED":"",i.doubleSided?"#define DOUBLE_SIDED":"",i.flipSided?"#define FLIP_SIDED":"",i.shadowMapEnabled?"#define USE_SHADOWMAP":"",i.shadowMapEnabled?"#define "+m:"",i.premultipliedAlpha?"#define PREMULTIPLIED_ALPHA":"",i.numLightProbes>0?"#define USE_LIGHT_PROBES":"",i.decodeVideoTexture?"#define DECODE_VIDEO_TEXTURE":"",i.decodeVideoTextureEmissive?"#define DECODE_VIDEO_TEXTURE_EMISSIVE":"",i.logarithmicDepthBuffer?"#define USE_LOGARITHMIC_DEPTH_BUFFER":"",i.reversedDepthBuffer?"#define USE_REVERSED_DEPTH_BUFFER":"","uniform mat4 viewMatrix;","uniform vec3 cameraPosition;","uniform bool isOrthographic;",i.toneMapping!==Vi?"#define TONE_MAPPING":"",i.toneMapping!==Vi?pt.tonemapping_pars_fragment:"",i.toneMapping!==Vi?x1("toneMapping",i.toneMapping):"",i.dithering?"#define DITHERING":"",i.opaque?"#define OPAQUE":"",pt.colorspace_pars_fragment,_1("linearToOutputTexel",i.outputColorSpace),y1(),i.useDepthPacking?"#define DEPTH_PACKING "+i.depthPacking:"",`
`].filter(Wo).join(`
`)),d=Hh(d),d=F_(d,i),d=z_(d,i),p=Hh(p),p=F_(p,i),p=z_(p,i),d=B_(d),p=B_(p),i.isRawShaderMaterial!==!0&&(C=`#version 300 es
`,b=[x,"#define attribute in","#define varying out","#define texture2D texture"].join(`
`)+`
`+b,S=["#define varying in",i.glslVersion===Q0?"":"layout(location = 0) out highp vec4 pc_fragColor;",i.glslVersion===Q0?"":"#define gl_FragColor pc_fragColor","#define gl_FragDepthEXT gl_FragDepth","#define texture2D texture","#define textureCube texture","#define texture2DProj textureProj","#define texture2DLodEXT textureLod","#define texture2DProjLodEXT textureProjLod","#define textureCubeLodEXT textureLod","#define texture2DGradEXT textureGrad","#define texture2DProjGradEXT textureProjGrad","#define textureCubeGradEXT textureGrad"].join(`
`)+`
`+S);const U=C+b+d,N=C+S+p,V=O_(l,l.VERTEX_SHADER,U),H=O_(l,l.FRAGMENT_SHADER,N);l.attachShader(w,V),l.attachShader(w,H),i.index0AttributeName!==void 0?l.bindAttribLocation(w,0,i.index0AttributeName):i.morphTargets===!0&&l.bindAttribLocation(w,0,"position"),l.linkProgram(w);function F(G){if(o.debug.checkShaderErrors){const te=l.getProgramInfoLog(w)||"",se=l.getShaderInfoLog(V)||"",ue=l.getShaderInfoLog(H)||"",ee=te.trim(),P=se.trim(),z=ue.trim();let ce=!0,pe=!0;if(l.getProgramParameter(w,l.LINK_STATUS)===!1)if(ce=!1,typeof o.debug.onShaderError=="function")o.debug.onShaderError(l,w,V,H);else{const Ee=I_(l,V,"vertex"),I=I_(l,H,"fragment");Dt("THREE.WebGLProgram: Shader Error "+l.getError()+" - VALIDATE_STATUS "+l.getProgramParameter(w,l.VALIDATE_STATUS)+`

Material Name: `+G.name+`
Material Type: `+G.type+`

Program Info Log: `+ee+`
`+Ee+`
`+I)}else ee!==""?at("WebGLProgram: Program Info Log:",ee):(P===""||z==="")&&(pe=!1);pe&&(G.diagnostics={runnable:ce,programLog:ee,vertexShader:{log:P,prefix:b},fragmentShader:{log:z,prefix:S}})}l.deleteShader(V),l.deleteShader(H),T=new qc(l,w),D=M1(l,w)}let T;this.getUniforms=function(){return T===void 0&&F(this),T};let D;this.getAttributes=function(){return D===void 0&&F(this),D};let le=i.rendererExtensionParallelShaderCompile===!1;return this.isReady=function(){return le===!1&&(le=l.getProgramParameter(w,h1)),le},this.destroy=function(){s.releaseStatesOfProgram(this),l.deleteProgram(w),this.program=void 0},this.type=i.shaderType,this.name=i.shaderName,this.id=p1++,this.cacheKey=e,this.usedTimes=1,this.program=w,this.vertexShader=V,this.fragmentShader=H,this}let B1=0;class H1{constructor(){this.shaderCache=new Map,this.materialCache=new Map}update(e){const i=e.vertexShader,s=e.fragmentShader,l=this._getShaderStage(i),c=this._getShaderStage(s),d=this._getShaderCacheForMaterial(e);return d.has(l)===!1&&(d.add(l),l.usedTimes++),d.has(c)===!1&&(d.add(c),c.usedTimes++),this}remove(e){const i=this.materialCache.get(e);for(const s of i)s.usedTimes--,s.usedTimes===0&&this.shaderCache.delete(s.code);return this.materialCache.delete(e),this}getVertexShaderID(e){return this._getShaderStage(e.vertexShader).id}getFragmentShaderID(e){return this._getShaderStage(e.fragmentShader).id}dispose(){this.shaderCache.clear(),this.materialCache.clear()}_getShaderCacheForMaterial(e){const i=this.materialCache;let s=i.get(e);return s===void 0&&(s=new Set,i.set(e,s)),s}_getShaderStage(e){const i=this.shaderCache;let s=i.get(e);return s===void 0&&(s=new G1(e),i.set(e,s)),s}}class G1{constructor(e){this.id=B1++,this.code=e,this.usedTimes=0}}function V1(o,e,i,s,l,c){const d=new gv,p=new H1,m=new Set,h=[],v=new Map,y=s.logarithmicDepthBuffer;let g=s.precision;const x={MeshDepthMaterial:"depth",MeshDistanceMaterial:"distance",MeshNormalMaterial:"normal",MeshBasicMaterial:"basic",MeshLambertMaterial:"lambert",MeshPhongMaterial:"phong",MeshToonMaterial:"toon",MeshStandardMaterial:"physical",MeshPhysicalMaterial:"physical",MeshMatcapMaterial:"matcap",LineBasicMaterial:"basic",LineDashedMaterial:"dashed",PointsMaterial:"points",ShadowMaterial:"shadow",SpriteMaterial:"sprite"};function E(T){return m.add(T),T===0?"uv":`uv${T}`}function w(T,D,le,G,te){const se=G.fog,ue=te.geometry,ee=T.isMeshStandardMaterial||T.isMeshLambertMaterial||T.isMeshPhongMaterial?G.environment:null,P=T.isMeshStandardMaterial||T.isMeshLambertMaterial&&!T.envMap||T.isMeshPhongMaterial&&!T.envMap,z=e.get(T.envMap||ee,P),ce=z&&z.mapping===Jc?z.image.height:null,pe=x[T.type];T.precision!==null&&(g=s.getMaxPrecision(T.precision),g!==T.precision&&at("WebGLProgram.getParameters:",T.precision,"not supported, using",g,"instead."));const Ee=ue.morphAttributes.position||ue.morphAttributes.normal||ue.morphAttributes.color,I=Ee!==void 0?Ee.length:0;let Y=0;ue.morphAttributes.position!==void 0&&(Y=1),ue.morphAttributes.normal!==void 0&&(Y=2),ue.morphAttributes.color!==void 0&&(Y=3);let ve,Re,Fe,ie;if(pe){const Et=Bi[pe];ve=Et.vertexShader,Re=Et.fragmentShader}else ve=T.vertexShader,Re=T.fragmentShader,p.update(T),Fe=p.getVertexShaderID(T),ie=p.getFragmentShaderID(T);const xe=o.getRenderTarget(),Te=o.state.buffers.depth.getReversed(),ke=te.isInstancedMesh===!0,Ke=te.isBatchedMesh===!0,$e=!!T.map,$t=!!T.matcap,xt=!!z,mt=!!T.aoMap,Nt=!!T.lightMap,ot=!!T.bumpMap,Qt=!!T.normalMap,k=!!T.displacementMap,qt=!!T.emissiveMap,Mt=!!T.metalnessMap,Lt=!!T.roughnessMap,We=T.anisotropy>0,L=T.clearcoat>0,M=T.dispersion>0,q=T.iridescence>0,me=T.sheen>0,ye=T.transmission>0,de=We&&!!T.anisotropyMap,Xe=L&&!!T.clearcoatMap,Ce=L&&!!T.clearcoatNormalMap,Ze=L&&!!T.clearcoatRoughnessMap,et=q&&!!T.iridescenceMap,Me=q&&!!T.iridescenceThicknessMap,Se=me&&!!T.sheenColorMap,Oe=me&&!!T.sheenRoughnessMap,Le=!!T.specularMap,Pe=!!T.specularColorMap,ut=!!T.specularIntensityMap,W=ye&&!!T.transmissionMap,we=ye&&!!T.thicknessMap,Ae=!!T.gradientMap,Ie=!!T.alphaMap,be=T.alphaTest>0,fe=!!T.alphaHash,Be=!!T.extensions;let nt=Vi;T.toneMapped&&(xe===null||xe.isXRRenderTarget===!0)&&(nt=o.toneMapping);const Pt={shaderID:pe,shaderType:T.type,shaderName:T.name,vertexShader:ve,fragmentShader:Re,defines:T.defines,customVertexShaderID:Fe,customFragmentShaderID:ie,isRawShaderMaterial:T.isRawShaderMaterial===!0,glslVersion:T.glslVersion,precision:g,batching:Ke,batchingColor:Ke&&te._colorsTexture!==null,instancing:ke,instancingColor:ke&&te.instanceColor!==null,instancingMorph:ke&&te.morphTexture!==null,outputColorSpace:xe===null?o.outputColorSpace:xe.isXRRenderTarget===!0?xe.texture.colorSpace:Vr,alphaToCoverage:!!T.alphaToCoverage,map:$e,matcap:$t,envMap:xt,envMapMode:xt&&z.mapping,envMapCubeUVHeight:ce,aoMap:mt,lightMap:Nt,bumpMap:ot,normalMap:Qt,displacementMap:k,emissiveMap:qt,normalMapObjectSpace:Qt&&T.normalMapType===MS,normalMapTangentSpace:Qt&&T.normalMapType===hv,metalnessMap:Mt,roughnessMap:Lt,anisotropy:We,anisotropyMap:de,clearcoat:L,clearcoatMap:Xe,clearcoatNormalMap:Ce,clearcoatRoughnessMap:Ze,dispersion:M,iridescence:q,iridescenceMap:et,iridescenceThicknessMap:Me,sheen:me,sheenColorMap:Se,sheenRoughnessMap:Oe,specularMap:Le,specularColorMap:Pe,specularIntensityMap:ut,transmission:ye,transmissionMap:W,thicknessMap:we,gradientMap:Ae,opaque:T.transparent===!1&&T.blending===Fr&&T.alphaToCoverage===!1,alphaMap:Ie,alphaTest:be,alphaHash:fe,combine:T.combine,mapUv:$e&&E(T.map.channel),aoMapUv:mt&&E(T.aoMap.channel),lightMapUv:Nt&&E(T.lightMap.channel),bumpMapUv:ot&&E(T.bumpMap.channel),normalMapUv:Qt&&E(T.normalMap.channel),displacementMapUv:k&&E(T.displacementMap.channel),emissiveMapUv:qt&&E(T.emissiveMap.channel),metalnessMapUv:Mt&&E(T.metalnessMap.channel),roughnessMapUv:Lt&&E(T.roughnessMap.channel),anisotropyMapUv:de&&E(T.anisotropyMap.channel),clearcoatMapUv:Xe&&E(T.clearcoatMap.channel),clearcoatNormalMapUv:Ce&&E(T.clearcoatNormalMap.channel),clearcoatRoughnessMapUv:Ze&&E(T.clearcoatRoughnessMap.channel),iridescenceMapUv:et&&E(T.iridescenceMap.channel),iridescenceThicknessMapUv:Me&&E(T.iridescenceThicknessMap.channel),sheenColorMapUv:Se&&E(T.sheenColorMap.channel),sheenRoughnessMapUv:Oe&&E(T.sheenRoughnessMap.channel),specularMapUv:Le&&E(T.specularMap.channel),specularColorMapUv:Pe&&E(T.specularColorMap.channel),specularIntensityMapUv:ut&&E(T.specularIntensityMap.channel),transmissionMapUv:W&&E(T.transmissionMap.channel),thicknessMapUv:we&&E(T.thicknessMap.channel),alphaMapUv:Ie&&E(T.alphaMap.channel),vertexTangents:!!ue.attributes.tangent&&(Qt||We),vertexColors:T.vertexColors,vertexAlphas:T.vertexColors===!0&&!!ue.attributes.color&&ue.attributes.color.itemSize===4,pointsUvs:te.isPoints===!0&&!!ue.attributes.uv&&($e||Ie),fog:!!se,useFog:T.fog===!0,fogExp2:!!se&&se.isFogExp2,flatShading:T.wireframe===!1&&(T.flatShading===!0||ue.attributes.normal===void 0&&Qt===!1&&(T.isMeshLambertMaterial||T.isMeshPhongMaterial||T.isMeshStandardMaterial||T.isMeshPhysicalMaterial)),sizeAttenuation:T.sizeAttenuation===!0,logarithmicDepthBuffer:y,reversedDepthBuffer:Te,skinning:te.isSkinnedMesh===!0,morphTargets:ue.morphAttributes.position!==void 0,morphNormals:ue.morphAttributes.normal!==void 0,morphColors:ue.morphAttributes.color!==void 0,morphTargetsCount:I,morphTextureStride:Y,numDirLights:D.directional.length,numPointLights:D.point.length,numSpotLights:D.spot.length,numSpotLightMaps:D.spotLightMap.length,numRectAreaLights:D.rectArea.length,numHemiLights:D.hemi.length,numDirLightShadows:D.directionalShadowMap.length,numPointLightShadows:D.pointShadowMap.length,numSpotLightShadows:D.spotShadowMap.length,numSpotLightShadowsWithMaps:D.numSpotLightShadowsWithMaps,numLightProbes:D.numLightProbes,numClippingPlanes:c.numPlanes,numClipIntersection:c.numIntersection,dithering:T.dithering,shadowMapEnabled:o.shadowMap.enabled&&le.length>0,shadowMapType:o.shadowMap.type,toneMapping:nt,decodeVideoTexture:$e&&T.map.isVideoTexture===!0&&Tt.getTransfer(T.map.colorSpace)===zt,decodeVideoTextureEmissive:qt&&T.emissiveMap.isVideoTexture===!0&&Tt.getTransfer(T.emissiveMap.colorSpace)===zt,premultipliedAlpha:T.premultipliedAlpha,doubleSided:T.side===va,flipSided:T.side===qn,useDepthPacking:T.depthPacking>=0,depthPacking:T.depthPacking||0,index0AttributeName:T.index0AttributeName,extensionClipCullDistance:Be&&T.extensions.clipCullDistance===!0&&i.has("WEBGL_clip_cull_distance"),extensionMultiDraw:(Be&&T.extensions.multiDraw===!0||Ke)&&i.has("WEBGL_multi_draw"),rendererExtensionParallelShaderCompile:i.has("KHR_parallel_shader_compile"),customProgramCacheKey:T.customProgramCacheKey()};return Pt.vertexUv1s=m.has(1),Pt.vertexUv2s=m.has(2),Pt.vertexUv3s=m.has(3),m.clear(),Pt}function b(T){const D=[];if(T.shaderID?D.push(T.shaderID):(D.push(T.customVertexShaderID),D.push(T.customFragmentShaderID)),T.defines!==void 0)for(const le in T.defines)D.push(le),D.push(T.defines[le]);return T.isRawShaderMaterial===!1&&(S(D,T),C(D,T),D.push(o.outputColorSpace)),D.push(T.customProgramCacheKey),D.join()}function S(T,D){T.push(D.precision),T.push(D.outputColorSpace),T.push(D.envMapMode),T.push(D.envMapCubeUVHeight),T.push(D.mapUv),T.push(D.alphaMapUv),T.push(D.lightMapUv),T.push(D.aoMapUv),T.push(D.bumpMapUv),T.push(D.normalMapUv),T.push(D.displacementMapUv),T.push(D.emissiveMapUv),T.push(D.metalnessMapUv),T.push(D.roughnessMapUv),T.push(D.anisotropyMapUv),T.push(D.clearcoatMapUv),T.push(D.clearcoatNormalMapUv),T.push(D.clearcoatRoughnessMapUv),T.push(D.iridescenceMapUv),T.push(D.iridescenceThicknessMapUv),T.push(D.sheenColorMapUv),T.push(D.sheenRoughnessMapUv),T.push(D.specularMapUv),T.push(D.specularColorMapUv),T.push(D.specularIntensityMapUv),T.push(D.transmissionMapUv),T.push(D.thicknessMapUv),T.push(D.combine),T.push(D.fogExp2),T.push(D.sizeAttenuation),T.push(D.morphTargetsCount),T.push(D.morphAttributeCount),T.push(D.numDirLights),T.push(D.numPointLights),T.push(D.numSpotLights),T.push(D.numSpotLightMaps),T.push(D.numHemiLights),T.push(D.numRectAreaLights),T.push(D.numDirLightShadows),T.push(D.numPointLightShadows),T.push(D.numSpotLightShadows),T.push(D.numSpotLightShadowsWithMaps),T.push(D.numLightProbes),T.push(D.shadowMapType),T.push(D.toneMapping),T.push(D.numClippingPlanes),T.push(D.numClipIntersection),T.push(D.depthPacking)}function C(T,D){d.disableAll(),D.instancing&&d.enable(0),D.instancingColor&&d.enable(1),D.instancingMorph&&d.enable(2),D.matcap&&d.enable(3),D.envMap&&d.enable(4),D.normalMapObjectSpace&&d.enable(5),D.normalMapTangentSpace&&d.enable(6),D.clearcoat&&d.enable(7),D.iridescence&&d.enable(8),D.alphaTest&&d.enable(9),D.vertexColors&&d.enable(10),D.vertexAlphas&&d.enable(11),D.vertexUv1s&&d.enable(12),D.vertexUv2s&&d.enable(13),D.vertexUv3s&&d.enable(14),D.vertexTangents&&d.enable(15),D.anisotropy&&d.enable(16),D.alphaHash&&d.enable(17),D.batching&&d.enable(18),D.dispersion&&d.enable(19),D.batchingColor&&d.enable(20),D.gradientMap&&d.enable(21),T.push(d.mask),d.disableAll(),D.fog&&d.enable(0),D.useFog&&d.enable(1),D.flatShading&&d.enable(2),D.logarithmicDepthBuffer&&d.enable(3),D.reversedDepthBuffer&&d.enable(4),D.skinning&&d.enable(5),D.morphTargets&&d.enable(6),D.morphNormals&&d.enable(7),D.morphColors&&d.enable(8),D.premultipliedAlpha&&d.enable(9),D.shadowMapEnabled&&d.enable(10),D.doubleSided&&d.enable(11),D.flipSided&&d.enable(12),D.useDepthPacking&&d.enable(13),D.dithering&&d.enable(14),D.transmission&&d.enable(15),D.sheen&&d.enable(16),D.opaque&&d.enable(17),D.pointsUvs&&d.enable(18),D.decodeVideoTexture&&d.enable(19),D.decodeVideoTextureEmissive&&d.enable(20),D.alphaToCoverage&&d.enable(21),T.push(d.mask)}function U(T){const D=x[T.type];let le;if(D){const G=Bi[D];le=lb.clone(G.uniforms)}else le=T.uniforms;return le}function N(T,D){let le=v.get(D);return le!==void 0?++le.usedTimes:(le=new z1(o,D,T,l),h.push(le),v.set(D,le)),le}function V(T){if(--T.usedTimes===0){const D=h.indexOf(T);h[D]=h[h.length-1],h.pop(),v.delete(T.cacheKey),T.destroy()}}function H(T){p.remove(T)}function F(){p.dispose()}return{getParameters:w,getProgramCacheKey:b,getUniforms:U,acquireProgram:N,releaseProgram:V,releaseShaderCache:H,programs:h,dispose:F}}function k1(){let o=new WeakMap;function e(d){return o.has(d)}function i(d){let p=o.get(d);return p===void 0&&(p={},o.set(d,p)),p}function s(d){o.delete(d)}function l(d,p,m){o.get(d)[p]=m}function c(){o=new WeakMap}return{has:e,get:i,remove:s,update:l,dispose:c}}function X1(o,e){return o.groupOrder!==e.groupOrder?o.groupOrder-e.groupOrder:o.renderOrder!==e.renderOrder?o.renderOrder-e.renderOrder:o.material.id!==e.material.id?o.material.id-e.material.id:o.materialVariant!==e.materialVariant?o.materialVariant-e.materialVariant:o.z!==e.z?o.z-e.z:o.id-e.id}function G_(o,e){return o.groupOrder!==e.groupOrder?o.groupOrder-e.groupOrder:o.renderOrder!==e.renderOrder?o.renderOrder-e.renderOrder:o.z!==e.z?e.z-o.z:o.id-e.id}function V_(){const o=[];let e=0;const i=[],s=[],l=[];function c(){e=0,i.length=0,s.length=0,l.length=0}function d(g){let x=0;return g.isInstancedMesh&&(x+=2),g.isSkinnedMesh&&(x+=1),x}function p(g,x,E,w,b,S){let C=o[e];return C===void 0?(C={id:g.id,object:g,geometry:x,material:E,materialVariant:d(g),groupOrder:w,renderOrder:g.renderOrder,z:b,group:S},o[e]=C):(C.id=g.id,C.object=g,C.geometry=x,C.material=E,C.materialVariant=d(g),C.groupOrder=w,C.renderOrder=g.renderOrder,C.z=b,C.group=S),e++,C}function m(g,x,E,w,b,S){const C=p(g,x,E,w,b,S);E.transmission>0?s.push(C):E.transparent===!0?l.push(C):i.push(C)}function h(g,x,E,w,b,S){const C=p(g,x,E,w,b,S);E.transmission>0?s.unshift(C):E.transparent===!0?l.unshift(C):i.unshift(C)}function v(g,x){i.length>1&&i.sort(g||X1),s.length>1&&s.sort(x||G_),l.length>1&&l.sort(x||G_)}function y(){for(let g=e,x=o.length;g<x;g++){const E=o[g];if(E.id===null)break;E.id=null,E.object=null,E.geometry=null,E.material=null,E.group=null}}return{opaque:i,transmissive:s,transparent:l,init:c,push:m,unshift:h,finish:y,sort:v}}function j1(){let o=new WeakMap;function e(s,l){const c=o.get(s);let d;return c===void 0?(d=new V_,o.set(s,[d])):l>=c.length?(d=new V_,c.push(d)):d=c[l],d}function i(){o=new WeakMap}return{get:e,dispose:i}}function W1(){const o={};return{get:function(e){if(o[e.id]!==void 0)return o[e.id];let i;switch(e.type){case"DirectionalLight":i={direction:new K,color:new At};break;case"SpotLight":i={position:new K,direction:new K,color:new At,distance:0,coneCos:0,penumbraCos:0,decay:0};break;case"PointLight":i={position:new K,color:new At,distance:0,decay:0};break;case"HemisphereLight":i={direction:new K,skyColor:new At,groundColor:new At};break;case"RectAreaLight":i={color:new At,position:new K,halfWidth:new K,halfHeight:new K};break}return o[e.id]=i,i}}}function q1(){const o={};return{get:function(e){if(o[e.id]!==void 0)return o[e.id];let i;switch(e.type){case"DirectionalLight":i={shadowIntensity:1,shadowBias:0,shadowNormalBias:0,shadowRadius:1,shadowMapSize:new ct};break;case"SpotLight":i={shadowIntensity:1,shadowBias:0,shadowNormalBias:0,shadowRadius:1,shadowMapSize:new ct};break;case"PointLight":i={shadowIntensity:1,shadowBias:0,shadowNormalBias:0,shadowRadius:1,shadowMapSize:new ct,shadowCameraNear:1,shadowCameraFar:1e3};break}return o[e.id]=i,i}}}let Y1=0;function Z1(o,e){return(e.castShadow?2:0)-(o.castShadow?2:0)+(e.map?1:0)-(o.map?1:0)}function K1(o){const e=new W1,i=q1(),s={version:0,hash:{directionalLength:-1,pointLength:-1,spotLength:-1,rectAreaLength:-1,hemiLength:-1,numDirectionalShadows:-1,numPointShadows:-1,numSpotShadows:-1,numSpotMaps:-1,numLightProbes:-1},ambient:[0,0,0],probe:[],directional:[],directionalShadow:[],directionalShadowMap:[],directionalShadowMatrix:[],spot:[],spotLightMap:[],spotShadow:[],spotShadowMap:[],spotLightMatrix:[],rectArea:[],rectAreaLTC1:null,rectAreaLTC2:null,point:[],pointShadow:[],pointShadowMap:[],pointShadowMatrix:[],hemi:[],numSpotLightShadowsWithMaps:0,numLightProbes:0};for(let h=0;h<9;h++)s.probe.push(new K);const l=new K,c=new Jt,d=new Jt;function p(h){let v=0,y=0,g=0;for(let D=0;D<9;D++)s.probe[D].set(0,0,0);let x=0,E=0,w=0,b=0,S=0,C=0,U=0,N=0,V=0,H=0,F=0;h.sort(Z1);for(let D=0,le=h.length;D<le;D++){const G=h[D],te=G.color,se=G.intensity,ue=G.distance;let ee=null;if(G.shadow&&G.shadow.map&&(G.shadow.map.texture.format===Gr?ee=G.shadow.map.texture:ee=G.shadow.map.depthTexture||G.shadow.map.texture),G.isAmbientLight)v+=te.r*se,y+=te.g*se,g+=te.b*se;else if(G.isLightProbe){for(let P=0;P<9;P++)s.probe[P].addScaledVector(G.sh.coefficients[P],se);F++}else if(G.isDirectionalLight){const P=e.get(G);if(P.color.copy(G.color).multiplyScalar(G.intensity),G.castShadow){const z=G.shadow,ce=i.get(G);ce.shadowIntensity=z.intensity,ce.shadowBias=z.bias,ce.shadowNormalBias=z.normalBias,ce.shadowRadius=z.radius,ce.shadowMapSize=z.mapSize,s.directionalShadow[x]=ce,s.directionalShadowMap[x]=ee,s.directionalShadowMatrix[x]=G.shadow.matrix,C++}s.directional[x]=P,x++}else if(G.isSpotLight){const P=e.get(G);P.position.setFromMatrixPosition(G.matrixWorld),P.color.copy(te).multiplyScalar(se),P.distance=ue,P.coneCos=Math.cos(G.angle),P.penumbraCos=Math.cos(G.angle*(1-G.penumbra)),P.decay=G.decay,s.spot[w]=P;const z=G.shadow;if(G.map&&(s.spotLightMap[V]=G.map,V++,z.updateMatrices(G),G.castShadow&&H++),s.spotLightMatrix[w]=z.matrix,G.castShadow){const ce=i.get(G);ce.shadowIntensity=z.intensity,ce.shadowBias=z.bias,ce.shadowNormalBias=z.normalBias,ce.shadowRadius=z.radius,ce.shadowMapSize=z.mapSize,s.spotShadow[w]=ce,s.spotShadowMap[w]=ee,N++}w++}else if(G.isRectAreaLight){const P=e.get(G);P.color.copy(te).multiplyScalar(se),P.halfWidth.set(G.width*.5,0,0),P.halfHeight.set(0,G.height*.5,0),s.rectArea[b]=P,b++}else if(G.isPointLight){const P=e.get(G);if(P.color.copy(G.color).multiplyScalar(G.intensity),P.distance=G.distance,P.decay=G.decay,G.castShadow){const z=G.shadow,ce=i.get(G);ce.shadowIntensity=z.intensity,ce.shadowBias=z.bias,ce.shadowNormalBias=z.normalBias,ce.shadowRadius=z.radius,ce.shadowMapSize=z.mapSize,ce.shadowCameraNear=z.camera.near,ce.shadowCameraFar=z.camera.far,s.pointShadow[E]=ce,s.pointShadowMap[E]=ee,s.pointShadowMatrix[E]=G.shadow.matrix,U++}s.point[E]=P,E++}else if(G.isHemisphereLight){const P=e.get(G);P.skyColor.copy(G.color).multiplyScalar(se),P.groundColor.copy(G.groundColor).multiplyScalar(se),s.hemi[S]=P,S++}}b>0&&(o.has("OES_texture_float_linear")===!0?(s.rectAreaLTC1=Ue.LTC_FLOAT_1,s.rectAreaLTC2=Ue.LTC_FLOAT_2):(s.rectAreaLTC1=Ue.LTC_HALF_1,s.rectAreaLTC2=Ue.LTC_HALF_2)),s.ambient[0]=v,s.ambient[1]=y,s.ambient[2]=g;const T=s.hash;(T.directionalLength!==x||T.pointLength!==E||T.spotLength!==w||T.rectAreaLength!==b||T.hemiLength!==S||T.numDirectionalShadows!==C||T.numPointShadows!==U||T.numSpotShadows!==N||T.numSpotMaps!==V||T.numLightProbes!==F)&&(s.directional.length=x,s.spot.length=w,s.rectArea.length=b,s.point.length=E,s.hemi.length=S,s.directionalShadow.length=C,s.directionalShadowMap.length=C,s.pointShadow.length=U,s.pointShadowMap.length=U,s.spotShadow.length=N,s.spotShadowMap.length=N,s.directionalShadowMatrix.length=C,s.pointShadowMatrix.length=U,s.spotLightMatrix.length=N+V-H,s.spotLightMap.length=V,s.numSpotLightShadowsWithMaps=H,s.numLightProbes=F,T.directionalLength=x,T.pointLength=E,T.spotLength=w,T.rectAreaLength=b,T.hemiLength=S,T.numDirectionalShadows=C,T.numPointShadows=U,T.numSpotShadows=N,T.numSpotMaps=V,T.numLightProbes=F,s.version=Y1++)}function m(h,v){let y=0,g=0,x=0,E=0,w=0;const b=v.matrixWorldInverse;for(let S=0,C=h.length;S<C;S++){const U=h[S];if(U.isDirectionalLight){const N=s.directional[y];N.direction.setFromMatrixPosition(U.matrixWorld),l.setFromMatrixPosition(U.target.matrixWorld),N.direction.sub(l),N.direction.transformDirection(b),y++}else if(U.isSpotLight){const N=s.spot[x];N.position.setFromMatrixPosition(U.matrixWorld),N.position.applyMatrix4(b),N.direction.setFromMatrixPosition(U.matrixWorld),l.setFromMatrixPosition(U.target.matrixWorld),N.direction.sub(l),N.direction.transformDirection(b),x++}else if(U.isRectAreaLight){const N=s.rectArea[E];N.position.setFromMatrixPosition(U.matrixWorld),N.position.applyMatrix4(b),d.identity(),c.copy(U.matrixWorld),c.premultiply(b),d.extractRotation(c),N.halfWidth.set(U.width*.5,0,0),N.halfHeight.set(0,U.height*.5,0),N.halfWidth.applyMatrix4(d),N.halfHeight.applyMatrix4(d),E++}else if(U.isPointLight){const N=s.point[g];N.position.setFromMatrixPosition(U.matrixWorld),N.position.applyMatrix4(b),g++}else if(U.isHemisphereLight){const N=s.hemi[w];N.direction.setFromMatrixPosition(U.matrixWorld),N.direction.transformDirection(b),w++}}}return{setup:p,setupView:m,state:s}}function k_(o){const e=new K1(o),i=[],s=[];function l(v){h.camera=v,i.length=0,s.length=0}function c(v){i.push(v)}function d(v){s.push(v)}function p(){e.setup(i)}function m(v){e.setupView(i,v)}const h={lightsArray:i,shadowsArray:s,camera:null,lights:e,transmissionRenderTarget:{}};return{init:l,state:h,setupLights:p,setupLightsView:m,pushLight:c,pushShadow:d}}function Q1(o){let e=new WeakMap;function i(l,c=0){const d=e.get(l);let p;return d===void 0?(p=new k_(o),e.set(l,[p])):c>=d.length?(p=new k_(o),d.push(p)):p=d[c],p}function s(){e=new WeakMap}return{get:i,dispose:s}}const J1=`void main() {
	gl_Position = vec4( position, 1.0 );
}`,$1=`uniform sampler2D shadow_pass;
uniform vec2 resolution;
uniform float radius;
void main() {
	const float samples = float( VSM_SAMPLES );
	float mean = 0.0;
	float squared_mean = 0.0;
	float uvStride = samples <= 1.0 ? 0.0 : 2.0 / ( samples - 1.0 );
	float uvStart = samples <= 1.0 ? 0.0 : - 1.0;
	for ( float i = 0.0; i < samples; i ++ ) {
		float uvOffset = uvStart + i * uvStride;
		#ifdef HORIZONTAL_PASS
			vec2 distribution = texture2D( shadow_pass, ( gl_FragCoord.xy + vec2( uvOffset, 0.0 ) * radius ) / resolution ).rg;
			mean += distribution.x;
			squared_mean += distribution.y * distribution.y + distribution.x * distribution.x;
		#else
			float depth = texture2D( shadow_pass, ( gl_FragCoord.xy + vec2( 0.0, uvOffset ) * radius ) / resolution ).r;
			mean += depth;
			squared_mean += depth * depth;
		#endif
	}
	mean = mean / samples;
	squared_mean = squared_mean / samples;
	float std_dev = sqrt( max( 0.0, squared_mean - mean * mean ) );
	gl_FragColor = vec4( mean, std_dev, 0.0, 1.0 );
}`,eA=[new K(1,0,0),new K(-1,0,0),new K(0,1,0),new K(0,-1,0),new K(0,0,1),new K(0,0,-1)],tA=[new K(0,-1,0),new K(0,-1,0),new K(0,0,1),new K(0,0,-1),new K(0,-1,0),new K(0,-1,0)],X_=new Jt,Xo=new K,jd=new K;function nA(o,e,i){let s=new tp;const l=new ct,c=new ct,d=new nn,p=new hb,m=new pb,h={},v=i.maxTextureSize,y={[ss]:qn,[qn]:ss,[va]:va},g=new qi({defines:{VSM_SAMPLES:8},uniforms:{shadow_pass:{value:null},resolution:{value:new ct},radius:{value:4}},vertexShader:J1,fragmentShader:$1}),x=g.clone();x.defines.HORIZONTAL_PASS=1;const E=new vi;E.setAttribute("position",new Ni(new Float32Array([-1,-1,.5,3,-1,.5,-1,3,.5]),3));const w=new Wi(E,g),b=this;this.enabled=!1,this.autoUpdate=!0,this.needsUpdate=!1,this.type=Gc;let S=this.type;this.render=function(H,F,T){if(b.enabled===!1||b.autoUpdate===!1&&b.needsUpdate===!1||H.length===0)return;this.type===eS&&(at("WebGLShadowMap: PCFSoftShadowMap has been deprecated. Using PCFShadowMap instead."),this.type=Gc);const D=o.getRenderTarget(),le=o.getActiveCubeFace(),G=o.getActiveMipmapLevel(),te=o.state;te.setBlending(ya),te.buffers.depth.getReversed()===!0?te.buffers.color.setClear(0,0,0,0):te.buffers.color.setClear(1,1,1,1),te.buffers.depth.setTest(!0),te.setScissorTest(!1);const se=S!==this.type;se&&F.traverse(function(ue){ue.material&&(Array.isArray(ue.material)?ue.material.forEach(ee=>ee.needsUpdate=!0):ue.material.needsUpdate=!0)});for(let ue=0,ee=H.length;ue<ee;ue++){const P=H[ue],z=P.shadow;if(z===void 0){at("WebGLShadowMap:",P,"has no shadow.");continue}if(z.autoUpdate===!1&&z.needsUpdate===!1)continue;l.copy(z.mapSize);const ce=z.getFrameExtents();l.multiply(ce),c.copy(z.mapSize),(l.x>v||l.y>v)&&(l.x>v&&(c.x=Math.floor(v/ce.x),l.x=c.x*ce.x,z.mapSize.x=c.x),l.y>v&&(c.y=Math.floor(v/ce.y),l.y=c.y*ce.y,z.mapSize.y=c.y));const pe=o.state.buffers.depth.getReversed();if(z.camera._reversedDepth=pe,z.map===null||se===!0){if(z.map!==null&&(z.map.depthTexture!==null&&(z.map.depthTexture.dispose(),z.map.depthTexture=null),z.map.dispose()),this.type===jo){if(P.isPointLight){at("WebGLShadowMap: VSM shadow maps are not supported for PointLights. Use PCF or BasicShadowMap instead.");continue}z.map=new ki(l.x,l.y,{format:Gr,type:ba,minFilter:Dn,magFilter:Dn,generateMipmaps:!1}),z.map.texture.name=P.name+".shadowMap",z.map.depthTexture=new Ko(l.x,l.y,Hi),z.map.depthTexture.name=P.name+".shadowMapDepth",z.map.depthTexture.format=Ma,z.map.depthTexture.compareFunction=null,z.map.depthTexture.minFilter=An,z.map.depthTexture.magFilter=An}else P.isPointLight?(z.map=new Cv(l.x),z.map.depthTexture=new rb(l.x,Xi)):(z.map=new ki(l.x,l.y),z.map.depthTexture=new Ko(l.x,l.y,Xi)),z.map.depthTexture.name=P.name+".shadowMap",z.map.depthTexture.format=Ma,this.type===Gc?(z.map.depthTexture.compareFunction=pe?Qh:Kh,z.map.depthTexture.minFilter=Dn,z.map.depthTexture.magFilter=Dn):(z.map.depthTexture.compareFunction=null,z.map.depthTexture.minFilter=An,z.map.depthTexture.magFilter=An);z.camera.updateProjectionMatrix()}const Ee=z.map.isWebGLCubeRenderTarget?6:1;for(let I=0;I<Ee;I++){if(z.map.isWebGLCubeRenderTarget)o.setRenderTarget(z.map,I),o.clear();else{I===0&&(o.setRenderTarget(z.map),o.clear());const Y=z.getViewport(I);d.set(c.x*Y.x,c.y*Y.y,c.x*Y.z,c.y*Y.w),te.viewport(d)}if(P.isPointLight){const Y=z.camera,ve=z.matrix,Re=P.distance||Y.far;Re!==Y.far&&(Y.far=Re,Y.updateProjectionMatrix()),Xo.setFromMatrixPosition(P.matrixWorld),Y.position.copy(Xo),jd.copy(Y.position),jd.add(eA[I]),Y.up.copy(tA[I]),Y.lookAt(jd),Y.updateMatrixWorld(),ve.makeTranslation(-Xo.x,-Xo.y,-Xo.z),X_.multiplyMatrices(Y.projectionMatrix,Y.matrixWorldInverse),z._frustum.setFromProjectionMatrix(X_,Y.coordinateSystem,Y.reversedDepth)}else z.updateMatrices(P);s=z.getFrustum(),N(F,T,z.camera,P,this.type)}z.isPointLightShadow!==!0&&this.type===jo&&C(z,T),z.needsUpdate=!1}S=this.type,b.needsUpdate=!1,o.setRenderTarget(D,le,G)};function C(H,F){const T=e.update(w);g.defines.VSM_SAMPLES!==H.blurSamples&&(g.defines.VSM_SAMPLES=H.blurSamples,x.defines.VSM_SAMPLES=H.blurSamples,g.needsUpdate=!0,x.needsUpdate=!0),H.mapPass===null&&(H.mapPass=new ki(l.x,l.y,{format:Gr,type:ba})),g.uniforms.shadow_pass.value=H.map.depthTexture,g.uniforms.resolution.value=H.mapSize,g.uniforms.radius.value=H.radius,o.setRenderTarget(H.mapPass),o.clear(),o.renderBufferDirect(F,null,T,g,w,null),x.uniforms.shadow_pass.value=H.mapPass.texture,x.uniforms.resolution.value=H.mapSize,x.uniforms.radius.value=H.radius,o.setRenderTarget(H.map),o.clear(),o.renderBufferDirect(F,null,T,x,w,null)}function U(H,F,T,D){let le=null;const G=T.isPointLight===!0?H.customDistanceMaterial:H.customDepthMaterial;if(G!==void 0)le=G;else if(le=T.isPointLight===!0?m:p,o.localClippingEnabled&&F.clipShadows===!0&&Array.isArray(F.clippingPlanes)&&F.clippingPlanes.length!==0||F.displacementMap&&F.displacementScale!==0||F.alphaMap&&F.alphaTest>0||F.map&&F.alphaTest>0||F.alphaToCoverage===!0){const te=le.uuid,se=F.uuid;let ue=h[te];ue===void 0&&(ue={},h[te]=ue);let ee=ue[se];ee===void 0&&(ee=le.clone(),ue[se]=ee,F.addEventListener("dispose",V)),le=ee}if(le.visible=F.visible,le.wireframe=F.wireframe,D===jo?le.side=F.shadowSide!==null?F.shadowSide:F.side:le.side=F.shadowSide!==null?F.shadowSide:y[F.side],le.alphaMap=F.alphaMap,le.alphaTest=F.alphaToCoverage===!0?.5:F.alphaTest,le.map=F.map,le.clipShadows=F.clipShadows,le.clippingPlanes=F.clippingPlanes,le.clipIntersection=F.clipIntersection,le.displacementMap=F.displacementMap,le.displacementScale=F.displacementScale,le.displacementBias=F.displacementBias,le.wireframeLinewidth=F.wireframeLinewidth,le.linewidth=F.linewidth,T.isPointLight===!0&&le.isMeshDistanceMaterial===!0){const te=o.properties.get(le);te.light=T}return le}function N(H,F,T,D,le){if(H.visible===!1)return;if(H.layers.test(F.layers)&&(H.isMesh||H.isLine||H.isPoints)&&(H.castShadow||H.receiveShadow&&le===jo)&&(!H.frustumCulled||s.intersectsObject(H))){H.modelViewMatrix.multiplyMatrices(T.matrixWorldInverse,H.matrixWorld);const se=e.update(H),ue=H.material;if(Array.isArray(ue)){const ee=se.groups;for(let P=0,z=ee.length;P<z;P++){const ce=ee[P],pe=ue[ce.materialIndex];if(pe&&pe.visible){const Ee=U(H,pe,D,le);H.onBeforeShadow(o,H,F,T,se,Ee,ce),o.renderBufferDirect(T,null,se,Ee,H,ce),H.onAfterShadow(o,H,F,T,se,Ee,ce)}}}else if(ue.visible){const ee=U(H,ue,D,le);H.onBeforeShadow(o,H,F,T,se,ee,null),o.renderBufferDirect(T,null,se,ee,H,null),H.onAfterShadow(o,H,F,T,se,ee,null)}}const te=H.children;for(let se=0,ue=te.length;se<ue;se++)N(te[se],F,T,D,le)}function V(H){H.target.removeEventListener("dispose",V);for(const T in h){const D=h[T],le=H.target.uuid;le in D&&(D[le].dispose(),delete D[le])}}}function iA(o,e){function i(){let W=!1;const we=new nn;let Ae=null;const Ie=new nn(0,0,0,0);return{setMask:function(be){Ae!==be&&!W&&(o.colorMask(be,be,be,be),Ae=be)},setLocked:function(be){W=be},setClear:function(be,fe,Be,nt,Pt){Pt===!0&&(be*=nt,fe*=nt,Be*=nt),we.set(be,fe,Be,nt),Ie.equals(we)===!1&&(o.clearColor(be,fe,Be,nt),Ie.copy(we))},reset:function(){W=!1,Ae=null,Ie.set(-1,0,0,0)}}}function s(){let W=!1,we=!1,Ae=null,Ie=null,be=null;return{setReversed:function(fe){if(we!==fe){const Be=e.get("EXT_clip_control");fe?Be.clipControlEXT(Be.LOWER_LEFT_EXT,Be.ZERO_TO_ONE_EXT):Be.clipControlEXT(Be.LOWER_LEFT_EXT,Be.NEGATIVE_ONE_TO_ONE_EXT),we=fe;const nt=be;be=null,this.setClear(nt)}},getReversed:function(){return we},setTest:function(fe){fe?xe(o.DEPTH_TEST):Te(o.DEPTH_TEST)},setMask:function(fe){Ae!==fe&&!W&&(o.depthMask(fe),Ae=fe)},setFunc:function(fe){if(we&&(fe=LS[fe]),Ie!==fe){switch(fe){case Kd:o.depthFunc(o.NEVER);break;case Qd:o.depthFunc(o.ALWAYS);break;case Jd:o.depthFunc(o.LESS);break;case Br:o.depthFunc(o.LEQUAL);break;case $d:o.depthFunc(o.EQUAL);break;case eh:o.depthFunc(o.GEQUAL);break;case th:o.depthFunc(o.GREATER);break;case nh:o.depthFunc(o.NOTEQUAL);break;default:o.depthFunc(o.LEQUAL)}Ie=fe}},setLocked:function(fe){W=fe},setClear:function(fe){be!==fe&&(be=fe,we&&(fe=1-fe),o.clearDepth(fe))},reset:function(){W=!1,Ae=null,Ie=null,be=null,we=!1}}}function l(){let W=!1,we=null,Ae=null,Ie=null,be=null,fe=null,Be=null,nt=null,Pt=null;return{setTest:function(Et){W||(Et?xe(o.STENCIL_TEST):Te(o.STENCIL_TEST))},setMask:function(Et){we!==Et&&!W&&(o.stencilMask(Et),we=Et)},setFunc:function(Et,Nn,xi){(Ae!==Et||Ie!==Nn||be!==xi)&&(o.stencilFunc(Et,Nn,xi),Ae=Et,Ie=Nn,be=xi)},setOp:function(Et,Nn,xi){(fe!==Et||Be!==Nn||nt!==xi)&&(o.stencilOp(Et,Nn,xi),fe=Et,Be=Nn,nt=xi)},setLocked:function(Et){W=Et},setClear:function(Et){Pt!==Et&&(o.clearStencil(Et),Pt=Et)},reset:function(){W=!1,we=null,Ae=null,Ie=null,be=null,fe=null,Be=null,nt=null,Pt=null}}}const c=new i,d=new s,p=new l,m=new WeakMap,h=new WeakMap;let v={},y={},g=new WeakMap,x=[],E=null,w=!1,b=null,S=null,C=null,U=null,N=null,V=null,H=null,F=new At(0,0,0),T=0,D=!1,le=null,G=null,te=null,se=null,ue=null;const ee=o.getParameter(o.MAX_COMBINED_TEXTURE_IMAGE_UNITS);let P=!1,z=0;const ce=o.getParameter(o.VERSION);ce.indexOf("WebGL")!==-1?(z=parseFloat(/^WebGL (\d)/.exec(ce)[1]),P=z>=1):ce.indexOf("OpenGL ES")!==-1&&(z=parseFloat(/^OpenGL ES (\d)/.exec(ce)[1]),P=z>=2);let pe=null,Ee={};const I=o.getParameter(o.SCISSOR_BOX),Y=o.getParameter(o.VIEWPORT),ve=new nn().fromArray(I),Re=new nn().fromArray(Y);function Fe(W,we,Ae,Ie){const be=new Uint8Array(4),fe=o.createTexture();o.bindTexture(W,fe),o.texParameteri(W,o.TEXTURE_MIN_FILTER,o.NEAREST),o.texParameteri(W,o.TEXTURE_MAG_FILTER,o.NEAREST);for(let Be=0;Be<Ae;Be++)W===o.TEXTURE_3D||W===o.TEXTURE_2D_ARRAY?o.texImage3D(we,0,o.RGBA,1,1,Ie,0,o.RGBA,o.UNSIGNED_BYTE,be):o.texImage2D(we+Be,0,o.RGBA,1,1,0,o.RGBA,o.UNSIGNED_BYTE,be);return fe}const ie={};ie[o.TEXTURE_2D]=Fe(o.TEXTURE_2D,o.TEXTURE_2D,1),ie[o.TEXTURE_CUBE_MAP]=Fe(o.TEXTURE_CUBE_MAP,o.TEXTURE_CUBE_MAP_POSITIVE_X,6),ie[o.TEXTURE_2D_ARRAY]=Fe(o.TEXTURE_2D_ARRAY,o.TEXTURE_2D_ARRAY,1,1),ie[o.TEXTURE_3D]=Fe(o.TEXTURE_3D,o.TEXTURE_3D,1,1),c.setClear(0,0,0,1),d.setClear(1),p.setClear(0),xe(o.DEPTH_TEST),d.setFunc(Br),ot(!1),Qt(W0),xe(o.CULL_FACE),mt(ya);function xe(W){v[W]!==!0&&(o.enable(W),v[W]=!0)}function Te(W){v[W]!==!1&&(o.disable(W),v[W]=!1)}function ke(W,we){return y[W]!==we?(o.bindFramebuffer(W,we),y[W]=we,W===o.DRAW_FRAMEBUFFER&&(y[o.FRAMEBUFFER]=we),W===o.FRAMEBUFFER&&(y[o.DRAW_FRAMEBUFFER]=we),!0):!1}function Ke(W,we){let Ae=x,Ie=!1;if(W){Ae=g.get(we),Ae===void 0&&(Ae=[],g.set(we,Ae));const be=W.textures;if(Ae.length!==be.length||Ae[0]!==o.COLOR_ATTACHMENT0){for(let fe=0,Be=be.length;fe<Be;fe++)Ae[fe]=o.COLOR_ATTACHMENT0+fe;Ae.length=be.length,Ie=!0}}else Ae[0]!==o.BACK&&(Ae[0]=o.BACK,Ie=!0);Ie&&o.drawBuffers(Ae)}function $e(W){return E!==W?(o.useProgram(W),E=W,!0):!1}const $t={[Us]:o.FUNC_ADD,[nS]:o.FUNC_SUBTRACT,[iS]:o.FUNC_REVERSE_SUBTRACT};$t[aS]=o.MIN,$t[sS]=o.MAX;const xt={[rS]:o.ZERO,[oS]:o.ONE,[lS]:o.SRC_COLOR,[Yd]:o.SRC_ALPHA,[pS]:o.SRC_ALPHA_SATURATE,[dS]:o.DST_COLOR,[uS]:o.DST_ALPHA,[cS]:o.ONE_MINUS_SRC_COLOR,[Zd]:o.ONE_MINUS_SRC_ALPHA,[hS]:o.ONE_MINUS_DST_COLOR,[fS]:o.ONE_MINUS_DST_ALPHA,[mS]:o.CONSTANT_COLOR,[gS]:o.ONE_MINUS_CONSTANT_COLOR,[_S]:o.CONSTANT_ALPHA,[vS]:o.ONE_MINUS_CONSTANT_ALPHA};function mt(W,we,Ae,Ie,be,fe,Be,nt,Pt,Et){if(W===ya){w===!0&&(Te(o.BLEND),w=!1);return}if(w===!1&&(xe(o.BLEND),w=!0),W!==tS){if(W!==b||Et!==D){if((S!==Us||N!==Us)&&(o.blendEquation(o.FUNC_ADD),S=Us,N=Us),Et)switch(W){case Fr:o.blendFuncSeparate(o.ONE,o.ONE_MINUS_SRC_ALPHA,o.ONE,o.ONE_MINUS_SRC_ALPHA);break;case qd:o.blendFunc(o.ONE,o.ONE);break;case q0:o.blendFuncSeparate(o.ZERO,o.ONE_MINUS_SRC_COLOR,o.ZERO,o.ONE);break;case Y0:o.blendFuncSeparate(o.DST_COLOR,o.ONE_MINUS_SRC_ALPHA,o.ZERO,o.ONE);break;default:Dt("WebGLState: Invalid blending: ",W);break}else switch(W){case Fr:o.blendFuncSeparate(o.SRC_ALPHA,o.ONE_MINUS_SRC_ALPHA,o.ONE,o.ONE_MINUS_SRC_ALPHA);break;case qd:o.blendFuncSeparate(o.SRC_ALPHA,o.ONE,o.ONE,o.ONE);break;case q0:Dt("WebGLState: SubtractiveBlending requires material.premultipliedAlpha = true");break;case Y0:Dt("WebGLState: MultiplyBlending requires material.premultipliedAlpha = true");break;default:Dt("WebGLState: Invalid blending: ",W);break}C=null,U=null,V=null,H=null,F.set(0,0,0),T=0,b=W,D=Et}return}be=be||we,fe=fe||Ae,Be=Be||Ie,(we!==S||be!==N)&&(o.blendEquationSeparate($t[we],$t[be]),S=we,N=be),(Ae!==C||Ie!==U||fe!==V||Be!==H)&&(o.blendFuncSeparate(xt[Ae],xt[Ie],xt[fe],xt[Be]),C=Ae,U=Ie,V=fe,H=Be),(nt.equals(F)===!1||Pt!==T)&&(o.blendColor(nt.r,nt.g,nt.b,Pt),F.copy(nt),T=Pt),b=W,D=!1}function Nt(W,we){W.side===va?Te(o.CULL_FACE):xe(o.CULL_FACE);let Ae=W.side===qn;we&&(Ae=!Ae),ot(Ae),W.blending===Fr&&W.transparent===!1?mt(ya):mt(W.blending,W.blendEquation,W.blendSrc,W.blendDst,W.blendEquationAlpha,W.blendSrcAlpha,W.blendDstAlpha,W.blendColor,W.blendAlpha,W.premultipliedAlpha),d.setFunc(W.depthFunc),d.setTest(W.depthTest),d.setMask(W.depthWrite),c.setMask(W.colorWrite);const Ie=W.stencilWrite;p.setTest(Ie),Ie&&(p.setMask(W.stencilWriteMask),p.setFunc(W.stencilFunc,W.stencilRef,W.stencilFuncMask),p.setOp(W.stencilFail,W.stencilZFail,W.stencilZPass)),qt(W.polygonOffset,W.polygonOffsetFactor,W.polygonOffsetUnits),W.alphaToCoverage===!0?xe(o.SAMPLE_ALPHA_TO_COVERAGE):Te(o.SAMPLE_ALPHA_TO_COVERAGE)}function ot(W){le!==W&&(W?o.frontFace(o.CW):o.frontFace(o.CCW),le=W)}function Qt(W){W!==Jy?(xe(o.CULL_FACE),W!==G&&(W===W0?o.cullFace(o.BACK):W===$y?o.cullFace(o.FRONT):o.cullFace(o.FRONT_AND_BACK))):Te(o.CULL_FACE),G=W}function k(W){W!==te&&(P&&o.lineWidth(W),te=W)}function qt(W,we,Ae){W?(xe(o.POLYGON_OFFSET_FILL),(se!==we||ue!==Ae)&&(se=we,ue=Ae,d.getReversed()&&(we=-we),o.polygonOffset(we,Ae))):Te(o.POLYGON_OFFSET_FILL)}function Mt(W){W?xe(o.SCISSOR_TEST):Te(o.SCISSOR_TEST)}function Lt(W){W===void 0&&(W=o.TEXTURE0+ee-1),pe!==W&&(o.activeTexture(W),pe=W)}function We(W,we,Ae){Ae===void 0&&(pe===null?Ae=o.TEXTURE0+ee-1:Ae=pe);let Ie=Ee[Ae];Ie===void 0&&(Ie={type:void 0,texture:void 0},Ee[Ae]=Ie),(Ie.type!==W||Ie.texture!==we)&&(pe!==Ae&&(o.activeTexture(Ae),pe=Ae),o.bindTexture(W,we||ie[W]),Ie.type=W,Ie.texture=we)}function L(){const W=Ee[pe];W!==void 0&&W.type!==void 0&&(o.bindTexture(W.type,null),W.type=void 0,W.texture=void 0)}function M(){try{o.compressedTexImage2D(...arguments)}catch(W){Dt("WebGLState:",W)}}function q(){try{o.compressedTexImage3D(...arguments)}catch(W){Dt("WebGLState:",W)}}function me(){try{o.texSubImage2D(...arguments)}catch(W){Dt("WebGLState:",W)}}function ye(){try{o.texSubImage3D(...arguments)}catch(W){Dt("WebGLState:",W)}}function de(){try{o.compressedTexSubImage2D(...arguments)}catch(W){Dt("WebGLState:",W)}}function Xe(){try{o.compressedTexSubImage3D(...arguments)}catch(W){Dt("WebGLState:",W)}}function Ce(){try{o.texStorage2D(...arguments)}catch(W){Dt("WebGLState:",W)}}function Ze(){try{o.texStorage3D(...arguments)}catch(W){Dt("WebGLState:",W)}}function et(){try{o.texImage2D(...arguments)}catch(W){Dt("WebGLState:",W)}}function Me(){try{o.texImage3D(...arguments)}catch(W){Dt("WebGLState:",W)}}function Se(W){ve.equals(W)===!1&&(o.scissor(W.x,W.y,W.z,W.w),ve.copy(W))}function Oe(W){Re.equals(W)===!1&&(o.viewport(W.x,W.y,W.z,W.w),Re.copy(W))}function Le(W,we){let Ae=h.get(we);Ae===void 0&&(Ae=new WeakMap,h.set(we,Ae));let Ie=Ae.get(W);Ie===void 0&&(Ie=o.getUniformBlockIndex(we,W.name),Ae.set(W,Ie))}function Pe(W,we){const Ie=h.get(we).get(W);m.get(we)!==Ie&&(o.uniformBlockBinding(we,Ie,W.__bindingPointIndex),m.set(we,Ie))}function ut(){o.disable(o.BLEND),o.disable(o.CULL_FACE),o.disable(o.DEPTH_TEST),o.disable(o.POLYGON_OFFSET_FILL),o.disable(o.SCISSOR_TEST),o.disable(o.STENCIL_TEST),o.disable(o.SAMPLE_ALPHA_TO_COVERAGE),o.blendEquation(o.FUNC_ADD),o.blendFunc(o.ONE,o.ZERO),o.blendFuncSeparate(o.ONE,o.ZERO,o.ONE,o.ZERO),o.blendColor(0,0,0,0),o.colorMask(!0,!0,!0,!0),o.clearColor(0,0,0,0),o.depthMask(!0),o.depthFunc(o.LESS),d.setReversed(!1),o.clearDepth(1),o.stencilMask(4294967295),o.stencilFunc(o.ALWAYS,0,4294967295),o.stencilOp(o.KEEP,o.KEEP,o.KEEP),o.clearStencil(0),o.cullFace(o.BACK),o.frontFace(o.CCW),o.polygonOffset(0,0),o.activeTexture(o.TEXTURE0),o.bindFramebuffer(o.FRAMEBUFFER,null),o.bindFramebuffer(o.DRAW_FRAMEBUFFER,null),o.bindFramebuffer(o.READ_FRAMEBUFFER,null),o.useProgram(null),o.lineWidth(1),o.scissor(0,0,o.canvas.width,o.canvas.height),o.viewport(0,0,o.canvas.width,o.canvas.height),v={},pe=null,Ee={},y={},g=new WeakMap,x=[],E=null,w=!1,b=null,S=null,C=null,U=null,N=null,V=null,H=null,F=new At(0,0,0),T=0,D=!1,le=null,G=null,te=null,se=null,ue=null,ve.set(0,0,o.canvas.width,o.canvas.height),Re.set(0,0,o.canvas.width,o.canvas.height),c.reset(),d.reset(),p.reset()}return{buffers:{color:c,depth:d,stencil:p},enable:xe,disable:Te,bindFramebuffer:ke,drawBuffers:Ke,useProgram:$e,setBlending:mt,setMaterial:Nt,setFlipSided:ot,setCullFace:Qt,setLineWidth:k,setPolygonOffset:qt,setScissorTest:Mt,activeTexture:Lt,bindTexture:We,unbindTexture:L,compressedTexImage2D:M,compressedTexImage3D:q,texImage2D:et,texImage3D:Me,updateUBOMapping:Le,uniformBlockBinding:Pe,texStorage2D:Ce,texStorage3D:Ze,texSubImage2D:me,texSubImage3D:ye,compressedTexSubImage2D:de,compressedTexSubImage3D:Xe,scissor:Se,viewport:Oe,reset:ut}}function aA(o,e,i,s,l,c,d){const p=e.has("WEBGL_multisampled_render_to_texture")?e.get("WEBGL_multisampled_render_to_texture"):null,m=typeof navigator>"u"?!1:/OculusBrowser/g.test(navigator.userAgent),h=new ct,v=new WeakMap;let y;const g=new WeakMap;let x=!1;try{x=typeof OffscreenCanvas<"u"&&new OffscreenCanvas(1,1).getContext("2d")!==null}catch{}function E(L,M){return x?new OffscreenCanvas(L,M):Zc("canvas")}function w(L,M,q){let me=1;const ye=We(L);if((ye.width>q||ye.height>q)&&(me=q/Math.max(ye.width,ye.height)),me<1)if(typeof HTMLImageElement<"u"&&L instanceof HTMLImageElement||typeof HTMLCanvasElement<"u"&&L instanceof HTMLCanvasElement||typeof ImageBitmap<"u"&&L instanceof ImageBitmap||typeof VideoFrame<"u"&&L instanceof VideoFrame){const de=Math.floor(me*ye.width),Xe=Math.floor(me*ye.height);y===void 0&&(y=E(de,Xe));const Ce=M?E(de,Xe):y;return Ce.width=de,Ce.height=Xe,Ce.getContext("2d").drawImage(L,0,0,de,Xe),at("WebGLRenderer: Texture has been resized from ("+ye.width+"x"+ye.height+") to ("+de+"x"+Xe+")."),Ce}else return"data"in L&&at("WebGLRenderer: Image in DataTexture is too big ("+ye.width+"x"+ye.height+")."),L;return L}function b(L){return L.generateMipmaps}function S(L){o.generateMipmap(L)}function C(L){return L.isWebGLCubeRenderTarget?o.TEXTURE_CUBE_MAP:L.isWebGL3DRenderTarget?o.TEXTURE_3D:L.isWebGLArrayRenderTarget||L.isCompressedArrayTexture?o.TEXTURE_2D_ARRAY:o.TEXTURE_2D}function U(L,M,q,me,ye=!1){if(L!==null){if(o[L]!==void 0)return o[L];at("WebGLRenderer: Attempt to use non-existing WebGL internal format '"+L+"'")}let de=M;if(M===o.RED&&(q===o.FLOAT&&(de=o.R32F),q===o.HALF_FLOAT&&(de=o.R16F),q===o.UNSIGNED_BYTE&&(de=o.R8)),M===o.RED_INTEGER&&(q===o.UNSIGNED_BYTE&&(de=o.R8UI),q===o.UNSIGNED_SHORT&&(de=o.R16UI),q===o.UNSIGNED_INT&&(de=o.R32UI),q===o.BYTE&&(de=o.R8I),q===o.SHORT&&(de=o.R16I),q===o.INT&&(de=o.R32I)),M===o.RG&&(q===o.FLOAT&&(de=o.RG32F),q===o.HALF_FLOAT&&(de=o.RG16F),q===o.UNSIGNED_BYTE&&(de=o.RG8)),M===o.RG_INTEGER&&(q===o.UNSIGNED_BYTE&&(de=o.RG8UI),q===o.UNSIGNED_SHORT&&(de=o.RG16UI),q===o.UNSIGNED_INT&&(de=o.RG32UI),q===o.BYTE&&(de=o.RG8I),q===o.SHORT&&(de=o.RG16I),q===o.INT&&(de=o.RG32I)),M===o.RGB_INTEGER&&(q===o.UNSIGNED_BYTE&&(de=o.RGB8UI),q===o.UNSIGNED_SHORT&&(de=o.RGB16UI),q===o.UNSIGNED_INT&&(de=o.RGB32UI),q===o.BYTE&&(de=o.RGB8I),q===o.SHORT&&(de=o.RGB16I),q===o.INT&&(de=o.RGB32I)),M===o.RGBA_INTEGER&&(q===o.UNSIGNED_BYTE&&(de=o.RGBA8UI),q===o.UNSIGNED_SHORT&&(de=o.RGBA16UI),q===o.UNSIGNED_INT&&(de=o.RGBA32UI),q===o.BYTE&&(de=o.RGBA8I),q===o.SHORT&&(de=o.RGBA16I),q===o.INT&&(de=o.RGBA32I)),M===o.RGB&&(q===o.UNSIGNED_INT_5_9_9_9_REV&&(de=o.RGB9_E5),q===o.UNSIGNED_INT_10F_11F_11F_REV&&(de=o.R11F_G11F_B10F)),M===o.RGBA){const Xe=ye?Yc:Tt.getTransfer(me);q===o.FLOAT&&(de=o.RGBA32F),q===o.HALF_FLOAT&&(de=o.RGBA16F),q===o.UNSIGNED_BYTE&&(de=Xe===zt?o.SRGB8_ALPHA8:o.RGBA8),q===o.UNSIGNED_SHORT_4_4_4_4&&(de=o.RGBA4),q===o.UNSIGNED_SHORT_5_5_5_1&&(de=o.RGB5_A1)}return(de===o.R16F||de===o.R32F||de===o.RG16F||de===o.RG32F||de===o.RGBA16F||de===o.RGBA32F)&&e.get("EXT_color_buffer_float"),de}function N(L,M){let q;return L?M===null||M===Xi||M===Yo?q=o.DEPTH24_STENCIL8:M===Hi?q=o.DEPTH32F_STENCIL8:M===qo&&(q=o.DEPTH24_STENCIL8,at("DepthTexture: 16 bit depth attachment is not supported with stencil. Using 24-bit attachment.")):M===null||M===Xi||M===Yo?q=o.DEPTH_COMPONENT24:M===Hi?q=o.DEPTH_COMPONENT32F:M===qo&&(q=o.DEPTH_COMPONENT16),q}function V(L,M){return b(L)===!0||L.isFramebufferTexture&&L.minFilter!==An&&L.minFilter!==Dn?Math.log2(Math.max(M.width,M.height))+1:L.mipmaps!==void 0&&L.mipmaps.length>0?L.mipmaps.length:L.isCompressedTexture&&Array.isArray(L.image)?M.mipmaps.length:1}function H(L){const M=L.target;M.removeEventListener("dispose",H),T(M),M.isVideoTexture&&v.delete(M)}function F(L){const M=L.target;M.removeEventListener("dispose",F),le(M)}function T(L){const M=s.get(L);if(M.__webglInit===void 0)return;const q=L.source,me=g.get(q);if(me){const ye=me[M.__cacheKey];ye.usedTimes--,ye.usedTimes===0&&D(L),Object.keys(me).length===0&&g.delete(q)}s.remove(L)}function D(L){const M=s.get(L);o.deleteTexture(M.__webglTexture);const q=L.source,me=g.get(q);delete me[M.__cacheKey],d.memory.textures--}function le(L){const M=s.get(L);if(L.depthTexture&&(L.depthTexture.dispose(),s.remove(L.depthTexture)),L.isWebGLCubeRenderTarget)for(let me=0;me<6;me++){if(Array.isArray(M.__webglFramebuffer[me]))for(let ye=0;ye<M.__webglFramebuffer[me].length;ye++)o.deleteFramebuffer(M.__webglFramebuffer[me][ye]);else o.deleteFramebuffer(M.__webglFramebuffer[me]);M.__webglDepthbuffer&&o.deleteRenderbuffer(M.__webglDepthbuffer[me])}else{if(Array.isArray(M.__webglFramebuffer))for(let me=0;me<M.__webglFramebuffer.length;me++)o.deleteFramebuffer(M.__webglFramebuffer[me]);else o.deleteFramebuffer(M.__webglFramebuffer);if(M.__webglDepthbuffer&&o.deleteRenderbuffer(M.__webglDepthbuffer),M.__webglMultisampledFramebuffer&&o.deleteFramebuffer(M.__webglMultisampledFramebuffer),M.__webglColorRenderbuffer)for(let me=0;me<M.__webglColorRenderbuffer.length;me++)M.__webglColorRenderbuffer[me]&&o.deleteRenderbuffer(M.__webglColorRenderbuffer[me]);M.__webglDepthRenderbuffer&&o.deleteRenderbuffer(M.__webglDepthRenderbuffer)}const q=L.textures;for(let me=0,ye=q.length;me<ye;me++){const de=s.get(q[me]);de.__webglTexture&&(o.deleteTexture(de.__webglTexture),d.memory.textures--),s.remove(q[me])}s.remove(L)}let G=0;function te(){G=0}function se(){const L=G;return L>=l.maxTextures&&at("WebGLTextures: Trying to use "+L+" texture units while this GPU supports only "+l.maxTextures),G+=1,L}function ue(L){const M=[];return M.push(L.wrapS),M.push(L.wrapT),M.push(L.wrapR||0),M.push(L.magFilter),M.push(L.minFilter),M.push(L.anisotropy),M.push(L.internalFormat),M.push(L.format),M.push(L.type),M.push(L.generateMipmaps),M.push(L.premultiplyAlpha),M.push(L.flipY),M.push(L.unpackAlignment),M.push(L.colorSpace),M.join()}function ee(L,M){const q=s.get(L);if(L.isVideoTexture&&Mt(L),L.isRenderTargetTexture===!1&&L.isExternalTexture!==!0&&L.version>0&&q.__version!==L.version){const me=L.image;if(me===null)at("WebGLRenderer: Texture marked for update but no image data found.");else if(me.complete===!1)at("WebGLRenderer: Texture marked for update but image is incomplete");else{ie(q,L,M);return}}else L.isExternalTexture&&(q.__webglTexture=L.sourceTexture?L.sourceTexture:null);i.bindTexture(o.TEXTURE_2D,q.__webglTexture,o.TEXTURE0+M)}function P(L,M){const q=s.get(L);if(L.isRenderTargetTexture===!1&&L.version>0&&q.__version!==L.version){ie(q,L,M);return}else L.isExternalTexture&&(q.__webglTexture=L.sourceTexture?L.sourceTexture:null);i.bindTexture(o.TEXTURE_2D_ARRAY,q.__webglTexture,o.TEXTURE0+M)}function z(L,M){const q=s.get(L);if(L.isRenderTargetTexture===!1&&L.version>0&&q.__version!==L.version){ie(q,L,M);return}i.bindTexture(o.TEXTURE_3D,q.__webglTexture,o.TEXTURE0+M)}function ce(L,M){const q=s.get(L);if(L.isCubeDepthTexture!==!0&&L.version>0&&q.__version!==L.version){xe(q,L,M);return}i.bindTexture(o.TEXTURE_CUBE_MAP,q.__webglTexture,o.TEXTURE0+M)}const pe={[ih]:o.REPEAT,[xa]:o.CLAMP_TO_EDGE,[ah]:o.MIRRORED_REPEAT},Ee={[An]:o.NEAREST,[SS]:o.NEAREST_MIPMAP_NEAREST,[hc]:o.NEAREST_MIPMAP_LINEAR,[Dn]:o.LINEAR,[md]:o.LINEAR_MIPMAP_NEAREST,[Os]:o.LINEAR_MIPMAP_LINEAR},I={[ES]:o.NEVER,[CS]:o.ALWAYS,[TS]:o.LESS,[Kh]:o.LEQUAL,[AS]:o.EQUAL,[Qh]:o.GEQUAL,[RS]:o.GREATER,[wS]:o.NOTEQUAL};function Y(L,M){if(M.type===Hi&&e.has("OES_texture_float_linear")===!1&&(M.magFilter===Dn||M.magFilter===md||M.magFilter===hc||M.magFilter===Os||M.minFilter===Dn||M.minFilter===md||M.minFilter===hc||M.minFilter===Os)&&at("WebGLRenderer: Unable to use linear filtering with floating point textures. OES_texture_float_linear not supported on this device."),o.texParameteri(L,o.TEXTURE_WRAP_S,pe[M.wrapS]),o.texParameteri(L,o.TEXTURE_WRAP_T,pe[M.wrapT]),(L===o.TEXTURE_3D||L===o.TEXTURE_2D_ARRAY)&&o.texParameteri(L,o.TEXTURE_WRAP_R,pe[M.wrapR]),o.texParameteri(L,o.TEXTURE_MAG_FILTER,Ee[M.magFilter]),o.texParameteri(L,o.TEXTURE_MIN_FILTER,Ee[M.minFilter]),M.compareFunction&&(o.texParameteri(L,o.TEXTURE_COMPARE_MODE,o.COMPARE_REF_TO_TEXTURE),o.texParameteri(L,o.TEXTURE_COMPARE_FUNC,I[M.compareFunction])),e.has("EXT_texture_filter_anisotropic")===!0){if(M.magFilter===An||M.minFilter!==hc&&M.minFilter!==Os||M.type===Hi&&e.has("OES_texture_float_linear")===!1)return;if(M.anisotropy>1||s.get(M).__currentAnisotropy){const q=e.get("EXT_texture_filter_anisotropic");o.texParameterf(L,q.TEXTURE_MAX_ANISOTROPY_EXT,Math.min(M.anisotropy,l.getMaxAnisotropy())),s.get(M).__currentAnisotropy=M.anisotropy}}}function ve(L,M){let q=!1;L.__webglInit===void 0&&(L.__webglInit=!0,M.addEventListener("dispose",H));const me=M.source;let ye=g.get(me);ye===void 0&&(ye={},g.set(me,ye));const de=ue(M);if(de!==L.__cacheKey){ye[de]===void 0&&(ye[de]={texture:o.createTexture(),usedTimes:0},d.memory.textures++,q=!0),ye[de].usedTimes++;const Xe=ye[L.__cacheKey];Xe!==void 0&&(ye[L.__cacheKey].usedTimes--,Xe.usedTimes===0&&D(M)),L.__cacheKey=de,L.__webglTexture=ye[de].texture}return q}function Re(L,M,q){return Math.floor(Math.floor(L/q)/M)}function Fe(L,M,q,me){const de=L.updateRanges;if(de.length===0)i.texSubImage2D(o.TEXTURE_2D,0,0,0,M.width,M.height,q,me,M.data);else{de.sort((Me,Se)=>Me.start-Se.start);let Xe=0;for(let Me=1;Me<de.length;Me++){const Se=de[Xe],Oe=de[Me],Le=Se.start+Se.count,Pe=Re(Oe.start,M.width,4),ut=Re(Se.start,M.width,4);Oe.start<=Le+1&&Pe===ut&&Re(Oe.start+Oe.count-1,M.width,4)===Pe?Se.count=Math.max(Se.count,Oe.start+Oe.count-Se.start):(++Xe,de[Xe]=Oe)}de.length=Xe+1;const Ce=o.getParameter(o.UNPACK_ROW_LENGTH),Ze=o.getParameter(o.UNPACK_SKIP_PIXELS),et=o.getParameter(o.UNPACK_SKIP_ROWS);o.pixelStorei(o.UNPACK_ROW_LENGTH,M.width);for(let Me=0,Se=de.length;Me<Se;Me++){const Oe=de[Me],Le=Math.floor(Oe.start/4),Pe=Math.ceil(Oe.count/4),ut=Le%M.width,W=Math.floor(Le/M.width),we=Pe,Ae=1;o.pixelStorei(o.UNPACK_SKIP_PIXELS,ut),o.pixelStorei(o.UNPACK_SKIP_ROWS,W),i.texSubImage2D(o.TEXTURE_2D,0,ut,W,we,Ae,q,me,M.data)}L.clearUpdateRanges(),o.pixelStorei(o.UNPACK_ROW_LENGTH,Ce),o.pixelStorei(o.UNPACK_SKIP_PIXELS,Ze),o.pixelStorei(o.UNPACK_SKIP_ROWS,et)}}function ie(L,M,q){let me=o.TEXTURE_2D;(M.isDataArrayTexture||M.isCompressedArrayTexture)&&(me=o.TEXTURE_2D_ARRAY),M.isData3DTexture&&(me=o.TEXTURE_3D);const ye=ve(L,M),de=M.source;i.bindTexture(me,L.__webglTexture,o.TEXTURE0+q);const Xe=s.get(de);if(de.version!==Xe.__version||ye===!0){i.activeTexture(o.TEXTURE0+q);const Ce=Tt.getPrimaries(Tt.workingColorSpace),Ze=M.colorSpace===is?null:Tt.getPrimaries(M.colorSpace),et=M.colorSpace===is||Ce===Ze?o.NONE:o.BROWSER_DEFAULT_WEBGL;o.pixelStorei(o.UNPACK_FLIP_Y_WEBGL,M.flipY),o.pixelStorei(o.UNPACK_PREMULTIPLY_ALPHA_WEBGL,M.premultiplyAlpha),o.pixelStorei(o.UNPACK_ALIGNMENT,M.unpackAlignment),o.pixelStorei(o.UNPACK_COLORSPACE_CONVERSION_WEBGL,et);let Me=w(M.image,!1,l.maxTextureSize);Me=Lt(M,Me);const Se=c.convert(M.format,M.colorSpace),Oe=c.convert(M.type);let Le=U(M.internalFormat,Se,Oe,M.colorSpace,M.isVideoTexture);Y(me,M);let Pe;const ut=M.mipmaps,W=M.isVideoTexture!==!0,we=Xe.__version===void 0||ye===!0,Ae=de.dataReady,Ie=V(M,Me);if(M.isDepthTexture)Le=N(M.format===Ps,M.type),we&&(W?i.texStorage2D(o.TEXTURE_2D,1,Le,Me.width,Me.height):i.texImage2D(o.TEXTURE_2D,0,Le,Me.width,Me.height,0,Se,Oe,null));else if(M.isDataTexture)if(ut.length>0){W&&we&&i.texStorage2D(o.TEXTURE_2D,Ie,Le,ut[0].width,ut[0].height);for(let be=0,fe=ut.length;be<fe;be++)Pe=ut[be],W?Ae&&i.texSubImage2D(o.TEXTURE_2D,be,0,0,Pe.width,Pe.height,Se,Oe,Pe.data):i.texImage2D(o.TEXTURE_2D,be,Le,Pe.width,Pe.height,0,Se,Oe,Pe.data);M.generateMipmaps=!1}else W?(we&&i.texStorage2D(o.TEXTURE_2D,Ie,Le,Me.width,Me.height),Ae&&Fe(M,Me,Se,Oe)):i.texImage2D(o.TEXTURE_2D,0,Le,Me.width,Me.height,0,Se,Oe,Me.data);else if(M.isCompressedTexture)if(M.isCompressedArrayTexture){W&&we&&i.texStorage3D(o.TEXTURE_2D_ARRAY,Ie,Le,ut[0].width,ut[0].height,Me.depth);for(let be=0,fe=ut.length;be<fe;be++)if(Pe=ut[be],M.format!==Di)if(Se!==null)if(W){if(Ae)if(M.layerUpdates.size>0){const Be=S_(Pe.width,Pe.height,M.format,M.type);for(const nt of M.layerUpdates){const Pt=Pe.data.subarray(nt*Be/Pe.data.BYTES_PER_ELEMENT,(nt+1)*Be/Pe.data.BYTES_PER_ELEMENT);i.compressedTexSubImage3D(o.TEXTURE_2D_ARRAY,be,0,0,nt,Pe.width,Pe.height,1,Se,Pt)}M.clearLayerUpdates()}else i.compressedTexSubImage3D(o.TEXTURE_2D_ARRAY,be,0,0,0,Pe.width,Pe.height,Me.depth,Se,Pe.data)}else i.compressedTexImage3D(o.TEXTURE_2D_ARRAY,be,Le,Pe.width,Pe.height,Me.depth,0,Pe.data,0,0);else at("WebGLRenderer: Attempt to load unsupported compressed texture format in .uploadTexture()");else W?Ae&&i.texSubImage3D(o.TEXTURE_2D_ARRAY,be,0,0,0,Pe.width,Pe.height,Me.depth,Se,Oe,Pe.data):i.texImage3D(o.TEXTURE_2D_ARRAY,be,Le,Pe.width,Pe.height,Me.depth,0,Se,Oe,Pe.data)}else{W&&we&&i.texStorage2D(o.TEXTURE_2D,Ie,Le,ut[0].width,ut[0].height);for(let be=0,fe=ut.length;be<fe;be++)Pe=ut[be],M.format!==Di?Se!==null?W?Ae&&i.compressedTexSubImage2D(o.TEXTURE_2D,be,0,0,Pe.width,Pe.height,Se,Pe.data):i.compressedTexImage2D(o.TEXTURE_2D,be,Le,Pe.width,Pe.height,0,Pe.data):at("WebGLRenderer: Attempt to load unsupported compressed texture format in .uploadTexture()"):W?Ae&&i.texSubImage2D(o.TEXTURE_2D,be,0,0,Pe.width,Pe.height,Se,Oe,Pe.data):i.texImage2D(o.TEXTURE_2D,be,Le,Pe.width,Pe.height,0,Se,Oe,Pe.data)}else if(M.isDataArrayTexture)if(W){if(we&&i.texStorage3D(o.TEXTURE_2D_ARRAY,Ie,Le,Me.width,Me.height,Me.depth),Ae)if(M.layerUpdates.size>0){const be=S_(Me.width,Me.height,M.format,M.type);for(const fe of M.layerUpdates){const Be=Me.data.subarray(fe*be/Me.data.BYTES_PER_ELEMENT,(fe+1)*be/Me.data.BYTES_PER_ELEMENT);i.texSubImage3D(o.TEXTURE_2D_ARRAY,0,0,0,fe,Me.width,Me.height,1,Se,Oe,Be)}M.clearLayerUpdates()}else i.texSubImage3D(o.TEXTURE_2D_ARRAY,0,0,0,0,Me.width,Me.height,Me.depth,Se,Oe,Me.data)}else i.texImage3D(o.TEXTURE_2D_ARRAY,0,Le,Me.width,Me.height,Me.depth,0,Se,Oe,Me.data);else if(M.isData3DTexture)W?(we&&i.texStorage3D(o.TEXTURE_3D,Ie,Le,Me.width,Me.height,Me.depth),Ae&&i.texSubImage3D(o.TEXTURE_3D,0,0,0,0,Me.width,Me.height,Me.depth,Se,Oe,Me.data)):i.texImage3D(o.TEXTURE_3D,0,Le,Me.width,Me.height,Me.depth,0,Se,Oe,Me.data);else if(M.isFramebufferTexture){if(we)if(W)i.texStorage2D(o.TEXTURE_2D,Ie,Le,Me.width,Me.height);else{let be=Me.width,fe=Me.height;for(let Be=0;Be<Ie;Be++)i.texImage2D(o.TEXTURE_2D,Be,Le,be,fe,0,Se,Oe,null),be>>=1,fe>>=1}}else if(ut.length>0){if(W&&we){const be=We(ut[0]);i.texStorage2D(o.TEXTURE_2D,Ie,Le,be.width,be.height)}for(let be=0,fe=ut.length;be<fe;be++)Pe=ut[be],W?Ae&&i.texSubImage2D(o.TEXTURE_2D,be,0,0,Se,Oe,Pe):i.texImage2D(o.TEXTURE_2D,be,Le,Se,Oe,Pe);M.generateMipmaps=!1}else if(W){if(we){const be=We(Me);i.texStorage2D(o.TEXTURE_2D,Ie,Le,be.width,be.height)}Ae&&i.texSubImage2D(o.TEXTURE_2D,0,0,0,Se,Oe,Me)}else i.texImage2D(o.TEXTURE_2D,0,Le,Se,Oe,Me);b(M)&&S(me),Xe.__version=de.version,M.onUpdate&&M.onUpdate(M)}L.__version=M.version}function xe(L,M,q){if(M.image.length!==6)return;const me=ve(L,M),ye=M.source;i.bindTexture(o.TEXTURE_CUBE_MAP,L.__webglTexture,o.TEXTURE0+q);const de=s.get(ye);if(ye.version!==de.__version||me===!0){i.activeTexture(o.TEXTURE0+q);const Xe=Tt.getPrimaries(Tt.workingColorSpace),Ce=M.colorSpace===is?null:Tt.getPrimaries(M.colorSpace),Ze=M.colorSpace===is||Xe===Ce?o.NONE:o.BROWSER_DEFAULT_WEBGL;o.pixelStorei(o.UNPACK_FLIP_Y_WEBGL,M.flipY),o.pixelStorei(o.UNPACK_PREMULTIPLY_ALPHA_WEBGL,M.premultiplyAlpha),o.pixelStorei(o.UNPACK_ALIGNMENT,M.unpackAlignment),o.pixelStorei(o.UNPACK_COLORSPACE_CONVERSION_WEBGL,Ze);const et=M.isCompressedTexture||M.image[0].isCompressedTexture,Me=M.image[0]&&M.image[0].isDataTexture,Se=[];for(let fe=0;fe<6;fe++)!et&&!Me?Se[fe]=w(M.image[fe],!0,l.maxCubemapSize):Se[fe]=Me?M.image[fe].image:M.image[fe],Se[fe]=Lt(M,Se[fe]);const Oe=Se[0],Le=c.convert(M.format,M.colorSpace),Pe=c.convert(M.type),ut=U(M.internalFormat,Le,Pe,M.colorSpace),W=M.isVideoTexture!==!0,we=de.__version===void 0||me===!0,Ae=ye.dataReady;let Ie=V(M,Oe);Y(o.TEXTURE_CUBE_MAP,M);let be;if(et){W&&we&&i.texStorage2D(o.TEXTURE_CUBE_MAP,Ie,ut,Oe.width,Oe.height);for(let fe=0;fe<6;fe++){be=Se[fe].mipmaps;for(let Be=0;Be<be.length;Be++){const nt=be[Be];M.format!==Di?Le!==null?W?Ae&&i.compressedTexSubImage2D(o.TEXTURE_CUBE_MAP_POSITIVE_X+fe,Be,0,0,nt.width,nt.height,Le,nt.data):i.compressedTexImage2D(o.TEXTURE_CUBE_MAP_POSITIVE_X+fe,Be,ut,nt.width,nt.height,0,nt.data):at("WebGLRenderer: Attempt to load unsupported compressed texture format in .setTextureCube()"):W?Ae&&i.texSubImage2D(o.TEXTURE_CUBE_MAP_POSITIVE_X+fe,Be,0,0,nt.width,nt.height,Le,Pe,nt.data):i.texImage2D(o.TEXTURE_CUBE_MAP_POSITIVE_X+fe,Be,ut,nt.width,nt.height,0,Le,Pe,nt.data)}}}else{if(be=M.mipmaps,W&&we){be.length>0&&Ie++;const fe=We(Se[0]);i.texStorage2D(o.TEXTURE_CUBE_MAP,Ie,ut,fe.width,fe.height)}for(let fe=0;fe<6;fe++)if(Me){W?Ae&&i.texSubImage2D(o.TEXTURE_CUBE_MAP_POSITIVE_X+fe,0,0,0,Se[fe].width,Se[fe].height,Le,Pe,Se[fe].data):i.texImage2D(o.TEXTURE_CUBE_MAP_POSITIVE_X+fe,0,ut,Se[fe].width,Se[fe].height,0,Le,Pe,Se[fe].data);for(let Be=0;Be<be.length;Be++){const Pt=be[Be].image[fe].image;W?Ae&&i.texSubImage2D(o.TEXTURE_CUBE_MAP_POSITIVE_X+fe,Be+1,0,0,Pt.width,Pt.height,Le,Pe,Pt.data):i.texImage2D(o.TEXTURE_CUBE_MAP_POSITIVE_X+fe,Be+1,ut,Pt.width,Pt.height,0,Le,Pe,Pt.data)}}else{W?Ae&&i.texSubImage2D(o.TEXTURE_CUBE_MAP_POSITIVE_X+fe,0,0,0,Le,Pe,Se[fe]):i.texImage2D(o.TEXTURE_CUBE_MAP_POSITIVE_X+fe,0,ut,Le,Pe,Se[fe]);for(let Be=0;Be<be.length;Be++){const nt=be[Be];W?Ae&&i.texSubImage2D(o.TEXTURE_CUBE_MAP_POSITIVE_X+fe,Be+1,0,0,Le,Pe,nt.image[fe]):i.texImage2D(o.TEXTURE_CUBE_MAP_POSITIVE_X+fe,Be+1,ut,Le,Pe,nt.image[fe])}}}b(M)&&S(o.TEXTURE_CUBE_MAP),de.__version=ye.version,M.onUpdate&&M.onUpdate(M)}L.__version=M.version}function Te(L,M,q,me,ye,de){const Xe=c.convert(q.format,q.colorSpace),Ce=c.convert(q.type),Ze=U(q.internalFormat,Xe,Ce,q.colorSpace),et=s.get(M),Me=s.get(q);if(Me.__renderTarget=M,!et.__hasExternalTextures){const Se=Math.max(1,M.width>>de),Oe=Math.max(1,M.height>>de);ye===o.TEXTURE_3D||ye===o.TEXTURE_2D_ARRAY?i.texImage3D(ye,de,Ze,Se,Oe,M.depth,0,Xe,Ce,null):i.texImage2D(ye,de,Ze,Se,Oe,0,Xe,Ce,null)}i.bindFramebuffer(o.FRAMEBUFFER,L),qt(M)?p.framebufferTexture2DMultisampleEXT(o.FRAMEBUFFER,me,ye,Me.__webglTexture,0,k(M)):(ye===o.TEXTURE_2D||ye>=o.TEXTURE_CUBE_MAP_POSITIVE_X&&ye<=o.TEXTURE_CUBE_MAP_NEGATIVE_Z)&&o.framebufferTexture2D(o.FRAMEBUFFER,me,ye,Me.__webglTexture,de),i.bindFramebuffer(o.FRAMEBUFFER,null)}function ke(L,M,q){if(o.bindRenderbuffer(o.RENDERBUFFER,L),M.depthBuffer){const me=M.depthTexture,ye=me&&me.isDepthTexture?me.type:null,de=N(M.stencilBuffer,ye),Xe=M.stencilBuffer?o.DEPTH_STENCIL_ATTACHMENT:o.DEPTH_ATTACHMENT;qt(M)?p.renderbufferStorageMultisampleEXT(o.RENDERBUFFER,k(M),de,M.width,M.height):q?o.renderbufferStorageMultisample(o.RENDERBUFFER,k(M),de,M.width,M.height):o.renderbufferStorage(o.RENDERBUFFER,de,M.width,M.height),o.framebufferRenderbuffer(o.FRAMEBUFFER,Xe,o.RENDERBUFFER,L)}else{const me=M.textures;for(let ye=0;ye<me.length;ye++){const de=me[ye],Xe=c.convert(de.format,de.colorSpace),Ce=c.convert(de.type),Ze=U(de.internalFormat,Xe,Ce,de.colorSpace);qt(M)?p.renderbufferStorageMultisampleEXT(o.RENDERBUFFER,k(M),Ze,M.width,M.height):q?o.renderbufferStorageMultisample(o.RENDERBUFFER,k(M),Ze,M.width,M.height):o.renderbufferStorage(o.RENDERBUFFER,Ze,M.width,M.height)}}o.bindRenderbuffer(o.RENDERBUFFER,null)}function Ke(L,M,q){const me=M.isWebGLCubeRenderTarget===!0;if(i.bindFramebuffer(o.FRAMEBUFFER,L),!(M.depthTexture&&M.depthTexture.isDepthTexture))throw new Error("renderTarget.depthTexture must be an instance of THREE.DepthTexture");const ye=s.get(M.depthTexture);if(ye.__renderTarget=M,(!ye.__webglTexture||M.depthTexture.image.width!==M.width||M.depthTexture.image.height!==M.height)&&(M.depthTexture.image.width=M.width,M.depthTexture.image.height=M.height,M.depthTexture.needsUpdate=!0),me){if(ye.__webglInit===void 0&&(ye.__webglInit=!0,M.depthTexture.addEventListener("dispose",H)),ye.__webglTexture===void 0){ye.__webglTexture=o.createTexture(),i.bindTexture(o.TEXTURE_CUBE_MAP,ye.__webglTexture),Y(o.TEXTURE_CUBE_MAP,M.depthTexture);const et=c.convert(M.depthTexture.format),Me=c.convert(M.depthTexture.type);let Se;M.depthTexture.format===Ma?Se=o.DEPTH_COMPONENT24:M.depthTexture.format===Ps&&(Se=o.DEPTH24_STENCIL8);for(let Oe=0;Oe<6;Oe++)o.texImage2D(o.TEXTURE_CUBE_MAP_POSITIVE_X+Oe,0,Se,M.width,M.height,0,et,Me,null)}}else ee(M.depthTexture,0);const de=ye.__webglTexture,Xe=k(M),Ce=me?o.TEXTURE_CUBE_MAP_POSITIVE_X+q:o.TEXTURE_2D,Ze=M.depthTexture.format===Ps?o.DEPTH_STENCIL_ATTACHMENT:o.DEPTH_ATTACHMENT;if(M.depthTexture.format===Ma)qt(M)?p.framebufferTexture2DMultisampleEXT(o.FRAMEBUFFER,Ze,Ce,de,0,Xe):o.framebufferTexture2D(o.FRAMEBUFFER,Ze,Ce,de,0);else if(M.depthTexture.format===Ps)qt(M)?p.framebufferTexture2DMultisampleEXT(o.FRAMEBUFFER,Ze,Ce,de,0,Xe):o.framebufferTexture2D(o.FRAMEBUFFER,Ze,Ce,de,0);else throw new Error("Unknown depthTexture format")}function $e(L){const M=s.get(L),q=L.isWebGLCubeRenderTarget===!0;if(M.__boundDepthTexture!==L.depthTexture){const me=L.depthTexture;if(M.__depthDisposeCallback&&M.__depthDisposeCallback(),me){const ye=()=>{delete M.__boundDepthTexture,delete M.__depthDisposeCallback,me.removeEventListener("dispose",ye)};me.addEventListener("dispose",ye),M.__depthDisposeCallback=ye}M.__boundDepthTexture=me}if(L.depthTexture&&!M.__autoAllocateDepthBuffer)if(q)for(let me=0;me<6;me++)Ke(M.__webglFramebuffer[me],L,me);else{const me=L.texture.mipmaps;me&&me.length>0?Ke(M.__webglFramebuffer[0],L,0):Ke(M.__webglFramebuffer,L,0)}else if(q){M.__webglDepthbuffer=[];for(let me=0;me<6;me++)if(i.bindFramebuffer(o.FRAMEBUFFER,M.__webglFramebuffer[me]),M.__webglDepthbuffer[me]===void 0)M.__webglDepthbuffer[me]=o.createRenderbuffer(),ke(M.__webglDepthbuffer[me],L,!1);else{const ye=L.stencilBuffer?o.DEPTH_STENCIL_ATTACHMENT:o.DEPTH_ATTACHMENT,de=M.__webglDepthbuffer[me];o.bindRenderbuffer(o.RENDERBUFFER,de),o.framebufferRenderbuffer(o.FRAMEBUFFER,ye,o.RENDERBUFFER,de)}}else{const me=L.texture.mipmaps;if(me&&me.length>0?i.bindFramebuffer(o.FRAMEBUFFER,M.__webglFramebuffer[0]):i.bindFramebuffer(o.FRAMEBUFFER,M.__webglFramebuffer),M.__webglDepthbuffer===void 0)M.__webglDepthbuffer=o.createRenderbuffer(),ke(M.__webglDepthbuffer,L,!1);else{const ye=L.stencilBuffer?o.DEPTH_STENCIL_ATTACHMENT:o.DEPTH_ATTACHMENT,de=M.__webglDepthbuffer;o.bindRenderbuffer(o.RENDERBUFFER,de),o.framebufferRenderbuffer(o.FRAMEBUFFER,ye,o.RENDERBUFFER,de)}}i.bindFramebuffer(o.FRAMEBUFFER,null)}function $t(L,M,q){const me=s.get(L);M!==void 0&&Te(me.__webglFramebuffer,L,L.texture,o.COLOR_ATTACHMENT0,o.TEXTURE_2D,0),q!==void 0&&$e(L)}function xt(L){const M=L.texture,q=s.get(L),me=s.get(M);L.addEventListener("dispose",F);const ye=L.textures,de=L.isWebGLCubeRenderTarget===!0,Xe=ye.length>1;if(Xe||(me.__webglTexture===void 0&&(me.__webglTexture=o.createTexture()),me.__version=M.version,d.memory.textures++),de){q.__webglFramebuffer=[];for(let Ce=0;Ce<6;Ce++)if(M.mipmaps&&M.mipmaps.length>0){q.__webglFramebuffer[Ce]=[];for(let Ze=0;Ze<M.mipmaps.length;Ze++)q.__webglFramebuffer[Ce][Ze]=o.createFramebuffer()}else q.__webglFramebuffer[Ce]=o.createFramebuffer()}else{if(M.mipmaps&&M.mipmaps.length>0){q.__webglFramebuffer=[];for(let Ce=0;Ce<M.mipmaps.length;Ce++)q.__webglFramebuffer[Ce]=o.createFramebuffer()}else q.__webglFramebuffer=o.createFramebuffer();if(Xe)for(let Ce=0,Ze=ye.length;Ce<Ze;Ce++){const et=s.get(ye[Ce]);et.__webglTexture===void 0&&(et.__webglTexture=o.createTexture(),d.memory.textures++)}if(L.samples>0&&qt(L)===!1){q.__webglMultisampledFramebuffer=o.createFramebuffer(),q.__webglColorRenderbuffer=[],i.bindFramebuffer(o.FRAMEBUFFER,q.__webglMultisampledFramebuffer);for(let Ce=0;Ce<ye.length;Ce++){const Ze=ye[Ce];q.__webglColorRenderbuffer[Ce]=o.createRenderbuffer(),o.bindRenderbuffer(o.RENDERBUFFER,q.__webglColorRenderbuffer[Ce]);const et=c.convert(Ze.format,Ze.colorSpace),Me=c.convert(Ze.type),Se=U(Ze.internalFormat,et,Me,Ze.colorSpace,L.isXRRenderTarget===!0),Oe=k(L);o.renderbufferStorageMultisample(o.RENDERBUFFER,Oe,Se,L.width,L.height),o.framebufferRenderbuffer(o.FRAMEBUFFER,o.COLOR_ATTACHMENT0+Ce,o.RENDERBUFFER,q.__webglColorRenderbuffer[Ce])}o.bindRenderbuffer(o.RENDERBUFFER,null),L.depthBuffer&&(q.__webglDepthRenderbuffer=o.createRenderbuffer(),ke(q.__webglDepthRenderbuffer,L,!0)),i.bindFramebuffer(o.FRAMEBUFFER,null)}}if(de){i.bindTexture(o.TEXTURE_CUBE_MAP,me.__webglTexture),Y(o.TEXTURE_CUBE_MAP,M);for(let Ce=0;Ce<6;Ce++)if(M.mipmaps&&M.mipmaps.length>0)for(let Ze=0;Ze<M.mipmaps.length;Ze++)Te(q.__webglFramebuffer[Ce][Ze],L,M,o.COLOR_ATTACHMENT0,o.TEXTURE_CUBE_MAP_POSITIVE_X+Ce,Ze);else Te(q.__webglFramebuffer[Ce],L,M,o.COLOR_ATTACHMENT0,o.TEXTURE_CUBE_MAP_POSITIVE_X+Ce,0);b(M)&&S(o.TEXTURE_CUBE_MAP),i.unbindTexture()}else if(Xe){for(let Ce=0,Ze=ye.length;Ce<Ze;Ce++){const et=ye[Ce],Me=s.get(et);let Se=o.TEXTURE_2D;(L.isWebGL3DRenderTarget||L.isWebGLArrayRenderTarget)&&(Se=L.isWebGL3DRenderTarget?o.TEXTURE_3D:o.TEXTURE_2D_ARRAY),i.bindTexture(Se,Me.__webglTexture),Y(Se,et),Te(q.__webglFramebuffer,L,et,o.COLOR_ATTACHMENT0+Ce,Se,0),b(et)&&S(Se)}i.unbindTexture()}else{let Ce=o.TEXTURE_2D;if((L.isWebGL3DRenderTarget||L.isWebGLArrayRenderTarget)&&(Ce=L.isWebGL3DRenderTarget?o.TEXTURE_3D:o.TEXTURE_2D_ARRAY),i.bindTexture(Ce,me.__webglTexture),Y(Ce,M),M.mipmaps&&M.mipmaps.length>0)for(let Ze=0;Ze<M.mipmaps.length;Ze++)Te(q.__webglFramebuffer[Ze],L,M,o.COLOR_ATTACHMENT0,Ce,Ze);else Te(q.__webglFramebuffer,L,M,o.COLOR_ATTACHMENT0,Ce,0);b(M)&&S(Ce),i.unbindTexture()}L.depthBuffer&&$e(L)}function mt(L){const M=L.textures;for(let q=0,me=M.length;q<me;q++){const ye=M[q];if(b(ye)){const de=C(L),Xe=s.get(ye).__webglTexture;i.bindTexture(de,Xe),S(de),i.unbindTexture()}}}const Nt=[],ot=[];function Qt(L){if(L.samples>0){if(qt(L)===!1){const M=L.textures,q=L.width,me=L.height;let ye=o.COLOR_BUFFER_BIT;const de=L.stencilBuffer?o.DEPTH_STENCIL_ATTACHMENT:o.DEPTH_ATTACHMENT,Xe=s.get(L),Ce=M.length>1;if(Ce)for(let et=0;et<M.length;et++)i.bindFramebuffer(o.FRAMEBUFFER,Xe.__webglMultisampledFramebuffer),o.framebufferRenderbuffer(o.FRAMEBUFFER,o.COLOR_ATTACHMENT0+et,o.RENDERBUFFER,null),i.bindFramebuffer(o.FRAMEBUFFER,Xe.__webglFramebuffer),o.framebufferTexture2D(o.DRAW_FRAMEBUFFER,o.COLOR_ATTACHMENT0+et,o.TEXTURE_2D,null,0);i.bindFramebuffer(o.READ_FRAMEBUFFER,Xe.__webglMultisampledFramebuffer);const Ze=L.texture.mipmaps;Ze&&Ze.length>0?i.bindFramebuffer(o.DRAW_FRAMEBUFFER,Xe.__webglFramebuffer[0]):i.bindFramebuffer(o.DRAW_FRAMEBUFFER,Xe.__webglFramebuffer);for(let et=0;et<M.length;et++){if(L.resolveDepthBuffer&&(L.depthBuffer&&(ye|=o.DEPTH_BUFFER_BIT),L.stencilBuffer&&L.resolveStencilBuffer&&(ye|=o.STENCIL_BUFFER_BIT)),Ce){o.framebufferRenderbuffer(o.READ_FRAMEBUFFER,o.COLOR_ATTACHMENT0,o.RENDERBUFFER,Xe.__webglColorRenderbuffer[et]);const Me=s.get(M[et]).__webglTexture;o.framebufferTexture2D(o.DRAW_FRAMEBUFFER,o.COLOR_ATTACHMENT0,o.TEXTURE_2D,Me,0)}o.blitFramebuffer(0,0,q,me,0,0,q,me,ye,o.NEAREST),m===!0&&(Nt.length=0,ot.length=0,Nt.push(o.COLOR_ATTACHMENT0+et),L.depthBuffer&&L.resolveDepthBuffer===!1&&(Nt.push(de),ot.push(de),o.invalidateFramebuffer(o.DRAW_FRAMEBUFFER,ot)),o.invalidateFramebuffer(o.READ_FRAMEBUFFER,Nt))}if(i.bindFramebuffer(o.READ_FRAMEBUFFER,null),i.bindFramebuffer(o.DRAW_FRAMEBUFFER,null),Ce)for(let et=0;et<M.length;et++){i.bindFramebuffer(o.FRAMEBUFFER,Xe.__webglMultisampledFramebuffer),o.framebufferRenderbuffer(o.FRAMEBUFFER,o.COLOR_ATTACHMENT0+et,o.RENDERBUFFER,Xe.__webglColorRenderbuffer[et]);const Me=s.get(M[et]).__webglTexture;i.bindFramebuffer(o.FRAMEBUFFER,Xe.__webglFramebuffer),o.framebufferTexture2D(o.DRAW_FRAMEBUFFER,o.COLOR_ATTACHMENT0+et,o.TEXTURE_2D,Me,0)}i.bindFramebuffer(o.DRAW_FRAMEBUFFER,Xe.__webglMultisampledFramebuffer)}else if(L.depthBuffer&&L.resolveDepthBuffer===!1&&m){const M=L.stencilBuffer?o.DEPTH_STENCIL_ATTACHMENT:o.DEPTH_ATTACHMENT;o.invalidateFramebuffer(o.DRAW_FRAMEBUFFER,[M])}}}function k(L){return Math.min(l.maxSamples,L.samples)}function qt(L){const M=s.get(L);return L.samples>0&&e.has("WEBGL_multisampled_render_to_texture")===!0&&M.__useRenderToTexture!==!1}function Mt(L){const M=d.render.frame;v.get(L)!==M&&(v.set(L,M),L.update())}function Lt(L,M){const q=L.colorSpace,me=L.format,ye=L.type;return L.isCompressedTexture===!0||L.isVideoTexture===!0||q!==Vr&&q!==is&&(Tt.getTransfer(q)===zt?(me!==Di||ye!==ri)&&at("WebGLTextures: sRGB encoded textures have to use RGBAFormat and UnsignedByteType."):Dt("WebGLTextures: Unsupported texture color space:",q)),M}function We(L){return typeof HTMLImageElement<"u"&&L instanceof HTMLImageElement?(h.width=L.naturalWidth||L.width,h.height=L.naturalHeight||L.height):typeof VideoFrame<"u"&&L instanceof VideoFrame?(h.width=L.displayWidth,h.height=L.displayHeight):(h.width=L.width,h.height=L.height),h}this.allocateTextureUnit=se,this.resetTextureUnits=te,this.setTexture2D=ee,this.setTexture2DArray=P,this.setTexture3D=z,this.setTextureCube=ce,this.rebindTextures=$t,this.setupRenderTarget=xt,this.updateRenderTargetMipmap=mt,this.updateMultisampleRenderTarget=Qt,this.setupDepthRenderbuffer=$e,this.setupFrameBufferTexture=Te,this.useMultisampledRTT=qt,this.isReversedDepthBuffer=function(){return i.buffers.depth.getReversed()}}function sA(o,e){function i(s,l=is){let c;const d=Tt.getTransfer(l);if(s===ri)return o.UNSIGNED_BYTE;if(s===jh)return o.UNSIGNED_SHORT_4_4_4_4;if(s===Wh)return o.UNSIGNED_SHORT_5_5_5_1;if(s===lv)return o.UNSIGNED_INT_5_9_9_9_REV;if(s===cv)return o.UNSIGNED_INT_10F_11F_11F_REV;if(s===rv)return o.BYTE;if(s===ov)return o.SHORT;if(s===qo)return o.UNSIGNED_SHORT;if(s===Xh)return o.INT;if(s===Xi)return o.UNSIGNED_INT;if(s===Hi)return o.FLOAT;if(s===ba)return o.HALF_FLOAT;if(s===uv)return o.ALPHA;if(s===fv)return o.RGB;if(s===Di)return o.RGBA;if(s===Ma)return o.DEPTH_COMPONENT;if(s===Ps)return o.DEPTH_STENCIL;if(s===dv)return o.RED;if(s===qh)return o.RED_INTEGER;if(s===Gr)return o.RG;if(s===Yh)return o.RG_INTEGER;if(s===Zh)return o.RGBA_INTEGER;if(s===Vc||s===kc||s===Xc||s===jc)if(d===zt)if(c=e.get("WEBGL_compressed_texture_s3tc_srgb"),c!==null){if(s===Vc)return c.COMPRESSED_SRGB_S3TC_DXT1_EXT;if(s===kc)return c.COMPRESSED_SRGB_ALPHA_S3TC_DXT1_EXT;if(s===Xc)return c.COMPRESSED_SRGB_ALPHA_S3TC_DXT3_EXT;if(s===jc)return c.COMPRESSED_SRGB_ALPHA_S3TC_DXT5_EXT}else return null;else if(c=e.get("WEBGL_compressed_texture_s3tc"),c!==null){if(s===Vc)return c.COMPRESSED_RGB_S3TC_DXT1_EXT;if(s===kc)return c.COMPRESSED_RGBA_S3TC_DXT1_EXT;if(s===Xc)return c.COMPRESSED_RGBA_S3TC_DXT3_EXT;if(s===jc)return c.COMPRESSED_RGBA_S3TC_DXT5_EXT}else return null;if(s===sh||s===rh||s===oh||s===lh)if(c=e.get("WEBGL_compressed_texture_pvrtc"),c!==null){if(s===sh)return c.COMPRESSED_RGB_PVRTC_4BPPV1_IMG;if(s===rh)return c.COMPRESSED_RGB_PVRTC_2BPPV1_IMG;if(s===oh)return c.COMPRESSED_RGBA_PVRTC_4BPPV1_IMG;if(s===lh)return c.COMPRESSED_RGBA_PVRTC_2BPPV1_IMG}else return null;if(s===ch||s===uh||s===fh||s===dh||s===hh||s===ph||s===mh)if(c=e.get("WEBGL_compressed_texture_etc"),c!==null){if(s===ch||s===uh)return d===zt?c.COMPRESSED_SRGB8_ETC2:c.COMPRESSED_RGB8_ETC2;if(s===fh)return d===zt?c.COMPRESSED_SRGB8_ALPHA8_ETC2_EAC:c.COMPRESSED_RGBA8_ETC2_EAC;if(s===dh)return c.COMPRESSED_R11_EAC;if(s===hh)return c.COMPRESSED_SIGNED_R11_EAC;if(s===ph)return c.COMPRESSED_RG11_EAC;if(s===mh)return c.COMPRESSED_SIGNED_RG11_EAC}else return null;if(s===gh||s===_h||s===vh||s===xh||s===yh||s===Sh||s===bh||s===Mh||s===Eh||s===Th||s===Ah||s===Rh||s===wh||s===Ch)if(c=e.get("WEBGL_compressed_texture_astc"),c!==null){if(s===gh)return d===zt?c.COMPRESSED_SRGB8_ALPHA8_ASTC_4x4_KHR:c.COMPRESSED_RGBA_ASTC_4x4_KHR;if(s===_h)return d===zt?c.COMPRESSED_SRGB8_ALPHA8_ASTC_5x4_KHR:c.COMPRESSED_RGBA_ASTC_5x4_KHR;if(s===vh)return d===zt?c.COMPRESSED_SRGB8_ALPHA8_ASTC_5x5_KHR:c.COMPRESSED_RGBA_ASTC_5x5_KHR;if(s===xh)return d===zt?c.COMPRESSED_SRGB8_ALPHA8_ASTC_6x5_KHR:c.COMPRESSED_RGBA_ASTC_6x5_KHR;if(s===yh)return d===zt?c.COMPRESSED_SRGB8_ALPHA8_ASTC_6x6_KHR:c.COMPRESSED_RGBA_ASTC_6x6_KHR;if(s===Sh)return d===zt?c.COMPRESSED_SRGB8_ALPHA8_ASTC_8x5_KHR:c.COMPRESSED_RGBA_ASTC_8x5_KHR;if(s===bh)return d===zt?c.COMPRESSED_SRGB8_ALPHA8_ASTC_8x6_KHR:c.COMPRESSED_RGBA_ASTC_8x6_KHR;if(s===Mh)return d===zt?c.COMPRESSED_SRGB8_ALPHA8_ASTC_8x8_KHR:c.COMPRESSED_RGBA_ASTC_8x8_KHR;if(s===Eh)return d===zt?c.COMPRESSED_SRGB8_ALPHA8_ASTC_10x5_KHR:c.COMPRESSED_RGBA_ASTC_10x5_KHR;if(s===Th)return d===zt?c.COMPRESSED_SRGB8_ALPHA8_ASTC_10x6_KHR:c.COMPRESSED_RGBA_ASTC_10x6_KHR;if(s===Ah)return d===zt?c.COMPRESSED_SRGB8_ALPHA8_ASTC_10x8_KHR:c.COMPRESSED_RGBA_ASTC_10x8_KHR;if(s===Rh)return d===zt?c.COMPRESSED_SRGB8_ALPHA8_ASTC_10x10_KHR:c.COMPRESSED_RGBA_ASTC_10x10_KHR;if(s===wh)return d===zt?c.COMPRESSED_SRGB8_ALPHA8_ASTC_12x10_KHR:c.COMPRESSED_RGBA_ASTC_12x10_KHR;if(s===Ch)return d===zt?c.COMPRESSED_SRGB8_ALPHA8_ASTC_12x12_KHR:c.COMPRESSED_RGBA_ASTC_12x12_KHR}else return null;if(s===Dh||s===Nh||s===Uh)if(c=e.get("EXT_texture_compression_bptc"),c!==null){if(s===Dh)return d===zt?c.COMPRESSED_SRGB_ALPHA_BPTC_UNORM_EXT:c.COMPRESSED_RGBA_BPTC_UNORM_EXT;if(s===Nh)return c.COMPRESSED_RGB_BPTC_SIGNED_FLOAT_EXT;if(s===Uh)return c.COMPRESSED_RGB_BPTC_UNSIGNED_FLOAT_EXT}else return null;if(s===Lh||s===Oh||s===Ph||s===Ih)if(c=e.get("EXT_texture_compression_rgtc"),c!==null){if(s===Lh)return c.COMPRESSED_RED_RGTC1_EXT;if(s===Oh)return c.COMPRESSED_SIGNED_RED_RGTC1_EXT;if(s===Ph)return c.COMPRESSED_RED_GREEN_RGTC2_EXT;if(s===Ih)return c.COMPRESSED_SIGNED_RED_GREEN_RGTC2_EXT}else return null;return s===Yo?o.UNSIGNED_INT_24_8:o[s]!==void 0?o[s]:null}return{convert:i}}const rA=`
void main() {

	gl_Position = vec4( position, 1.0 );

}`,oA=`
uniform sampler2DArray depthColor;
uniform float depthWidth;
uniform float depthHeight;

void main() {

	vec2 coord = vec2( gl_FragCoord.x / depthWidth, gl_FragCoord.y / depthHeight );

	if ( coord.x >= 1.0 ) {

		gl_FragDepth = texture( depthColor, vec3( coord.x - 1.0, coord.y, 1 ) ).r;

	} else {

		gl_FragDepth = texture( depthColor, vec3( coord.x, coord.y, 0 ) ).r;

	}

}`;class lA{constructor(){this.texture=null,this.mesh=null,this.depthNear=0,this.depthFar=0}init(e,i){if(this.texture===null){const s=new Mv(e.texture);(e.depthNear!==i.depthNear||e.depthFar!==i.depthFar)&&(this.depthNear=e.depthNear,this.depthFar=e.depthFar),this.texture=s}}getMesh(e){if(this.texture!==null&&this.mesh===null){const i=e.cameras[0].viewport,s=new qi({vertexShader:rA,fragmentShader:oA,uniforms:{depthColor:{value:this.texture},depthWidth:{value:i.z},depthHeight:{value:i.w}}});this.mesh=new Wi(new eu(20,20),s)}return this.mesh}reset(){this.texture=null,this.mesh=null}getDepthTexture(){return this.texture}}class cA extends Fs{constructor(e,i){super();const s=this;let l=null,c=1,d=null,p="local-floor",m=1,h=null,v=null,y=null,g=null,x=null,E=null;const w=typeof XRWebGLBinding<"u",b=new lA,S={},C=i.getContextAttributes();let U=null,N=null;const V=[],H=[],F=new ct;let T=null;const D=new si;D.viewport=new nn;const le=new si;le.viewport=new nn;const G=[D,le],te=new yb;let se=null,ue=null;this.cameraAutoUpdate=!0,this.enabled=!1,this.isPresenting=!1,this.getController=function(ie){let xe=V[ie];return xe===void 0&&(xe=new bd,V[ie]=xe),xe.getTargetRaySpace()},this.getControllerGrip=function(ie){let xe=V[ie];return xe===void 0&&(xe=new bd,V[ie]=xe),xe.getGripSpace()},this.getHand=function(ie){let xe=V[ie];return xe===void 0&&(xe=new bd,V[ie]=xe),xe.getHandSpace()};function ee(ie){const xe=H.indexOf(ie.inputSource);if(xe===-1)return;const Te=V[xe];Te!==void 0&&(Te.update(ie.inputSource,ie.frame,h||d),Te.dispatchEvent({type:ie.type,data:ie.inputSource}))}function P(){l.removeEventListener("select",ee),l.removeEventListener("selectstart",ee),l.removeEventListener("selectend",ee),l.removeEventListener("squeeze",ee),l.removeEventListener("squeezestart",ee),l.removeEventListener("squeezeend",ee),l.removeEventListener("end",P),l.removeEventListener("inputsourceschange",z);for(let ie=0;ie<V.length;ie++){const xe=H[ie];xe!==null&&(H[ie]=null,V[ie].disconnect(xe))}se=null,ue=null,b.reset();for(const ie in S)delete S[ie];e.setRenderTarget(U),x=null,g=null,y=null,l=null,N=null,Fe.stop(),s.isPresenting=!1,e.setPixelRatio(T),e.setSize(F.width,F.height,!1),s.dispatchEvent({type:"sessionend"})}this.setFramebufferScaleFactor=function(ie){c=ie,s.isPresenting===!0&&at("WebXRManager: Cannot change framebuffer scale while presenting.")},this.setReferenceSpaceType=function(ie){p=ie,s.isPresenting===!0&&at("WebXRManager: Cannot change reference space type while presenting.")},this.getReferenceSpace=function(){return h||d},this.setReferenceSpace=function(ie){h=ie},this.getBaseLayer=function(){return g!==null?g:x},this.getBinding=function(){return y===null&&w&&(y=new XRWebGLBinding(l,i)),y},this.getFrame=function(){return E},this.getSession=function(){return l},this.setSession=async function(ie){if(l=ie,l!==null){if(U=e.getRenderTarget(),l.addEventListener("select",ee),l.addEventListener("selectstart",ee),l.addEventListener("selectend",ee),l.addEventListener("squeeze",ee),l.addEventListener("squeezestart",ee),l.addEventListener("squeezeend",ee),l.addEventListener("end",P),l.addEventListener("inputsourceschange",z),C.xrCompatible!==!0&&await i.makeXRCompatible(),T=e.getPixelRatio(),e.getSize(F),w&&"createProjectionLayer"in XRWebGLBinding.prototype){let Te=null,ke=null,Ke=null;C.depth&&(Ke=C.stencil?i.DEPTH24_STENCIL8:i.DEPTH_COMPONENT24,Te=C.stencil?Ps:Ma,ke=C.stencil?Yo:Xi);const $e={colorFormat:i.RGBA8,depthFormat:Ke,scaleFactor:c};y=this.getBinding(),g=y.createProjectionLayer($e),l.updateRenderState({layers:[g]}),e.setPixelRatio(1),e.setSize(g.textureWidth,g.textureHeight,!1),N=new ki(g.textureWidth,g.textureHeight,{format:Di,type:ri,depthTexture:new Ko(g.textureWidth,g.textureHeight,ke,void 0,void 0,void 0,void 0,void 0,void 0,Te),stencilBuffer:C.stencil,colorSpace:e.outputColorSpace,samples:C.antialias?4:0,resolveDepthBuffer:g.ignoreDepthValues===!1,resolveStencilBuffer:g.ignoreDepthValues===!1})}else{const Te={antialias:C.antialias,alpha:!0,depth:C.depth,stencil:C.stencil,framebufferScaleFactor:c};x=new XRWebGLLayer(l,i,Te),l.updateRenderState({baseLayer:x}),e.setPixelRatio(1),e.setSize(x.framebufferWidth,x.framebufferHeight,!1),N=new ki(x.framebufferWidth,x.framebufferHeight,{format:Di,type:ri,colorSpace:e.outputColorSpace,stencilBuffer:C.stencil,resolveDepthBuffer:x.ignoreDepthValues===!1,resolveStencilBuffer:x.ignoreDepthValues===!1})}N.isXRRenderTarget=!0,this.setFoveation(m),h=null,d=await l.requestReferenceSpace(p),Fe.setContext(l),Fe.start(),s.isPresenting=!0,s.dispatchEvent({type:"sessionstart"})}},this.getEnvironmentBlendMode=function(){if(l!==null)return l.environmentBlendMode},this.getDepthTexture=function(){return b.getDepthTexture()};function z(ie){for(let xe=0;xe<ie.removed.length;xe++){const Te=ie.removed[xe],ke=H.indexOf(Te);ke>=0&&(H[ke]=null,V[ke].disconnect(Te))}for(let xe=0;xe<ie.added.length;xe++){const Te=ie.added[xe];let ke=H.indexOf(Te);if(ke===-1){for(let $e=0;$e<V.length;$e++)if($e>=H.length){H.push(Te),ke=$e;break}else if(H[$e]===null){H[$e]=Te,ke=$e;break}if(ke===-1)break}const Ke=V[ke];Ke&&Ke.connect(Te)}}const ce=new K,pe=new K;function Ee(ie,xe,Te){ce.setFromMatrixPosition(xe.matrixWorld),pe.setFromMatrixPosition(Te.matrixWorld);const ke=ce.distanceTo(pe),Ke=xe.projectionMatrix.elements,$e=Te.projectionMatrix.elements,$t=Ke[14]/(Ke[10]-1),xt=Ke[14]/(Ke[10]+1),mt=(Ke[9]+1)/Ke[5],Nt=(Ke[9]-1)/Ke[5],ot=(Ke[8]-1)/Ke[0],Qt=($e[8]+1)/$e[0],k=$t*ot,qt=$t*Qt,Mt=ke/(-ot+Qt),Lt=Mt*-ot;if(xe.matrixWorld.decompose(ie.position,ie.quaternion,ie.scale),ie.translateX(Lt),ie.translateZ(Mt),ie.matrixWorld.compose(ie.position,ie.quaternion,ie.scale),ie.matrixWorldInverse.copy(ie.matrixWorld).invert(),Ke[10]===-1)ie.projectionMatrix.copy(xe.projectionMatrix),ie.projectionMatrixInverse.copy(xe.projectionMatrixInverse);else{const We=$t+Mt,L=xt+Mt,M=k-Lt,q=qt+(ke-Lt),me=mt*xt/L*We,ye=Nt*xt/L*We;ie.projectionMatrix.makePerspective(M,q,me,ye,We,L),ie.projectionMatrixInverse.copy(ie.projectionMatrix).invert()}}function I(ie,xe){xe===null?ie.matrixWorld.copy(ie.matrix):ie.matrixWorld.multiplyMatrices(xe.matrixWorld,ie.matrix),ie.matrixWorldInverse.copy(ie.matrixWorld).invert()}this.updateCamera=function(ie){if(l===null)return;let xe=ie.near,Te=ie.far;b.texture!==null&&(b.depthNear>0&&(xe=b.depthNear),b.depthFar>0&&(Te=b.depthFar)),te.near=le.near=D.near=xe,te.far=le.far=D.far=Te,(se!==te.near||ue!==te.far)&&(l.updateRenderState({depthNear:te.near,depthFar:te.far}),se=te.near,ue=te.far),te.layers.mask=ie.layers.mask|6,D.layers.mask=te.layers.mask&-5,le.layers.mask=te.layers.mask&-3;const ke=ie.parent,Ke=te.cameras;I(te,ke);for(let $e=0;$e<Ke.length;$e++)I(Ke[$e],ke);Ke.length===2?Ee(te,D,le):te.projectionMatrix.copy(D.projectionMatrix),Y(ie,te,ke)};function Y(ie,xe,Te){Te===null?ie.matrix.copy(xe.matrixWorld):(ie.matrix.copy(Te.matrixWorld),ie.matrix.invert(),ie.matrix.multiply(xe.matrixWorld)),ie.matrix.decompose(ie.position,ie.quaternion,ie.scale),ie.updateMatrixWorld(!0),ie.projectionMatrix.copy(xe.projectionMatrix),ie.projectionMatrixInverse.copy(xe.projectionMatrixInverse),ie.isPerspectiveCamera&&(ie.fov=Fh*2*Math.atan(1/ie.projectionMatrix.elements[5]),ie.zoom=1)}this.getCamera=function(){return te},this.getFoveation=function(){if(!(g===null&&x===null))return m},this.setFoveation=function(ie){m=ie,g!==null&&(g.fixedFoveation=ie),x!==null&&x.fixedFoveation!==void 0&&(x.fixedFoveation=ie)},this.hasDepthSensing=function(){return b.texture!==null},this.getDepthSensingMesh=function(){return b.getMesh(te)},this.getCameraTexture=function(ie){return S[ie]};let ve=null;function Re(ie,xe){if(v=xe.getViewerPose(h||d),E=xe,v!==null){const Te=v.views;x!==null&&(e.setRenderTargetFramebuffer(N,x.framebuffer),e.setRenderTarget(N));let ke=!1;Te.length!==te.cameras.length&&(te.cameras.length=0,ke=!0);for(let xt=0;xt<Te.length;xt++){const mt=Te[xt];let Nt=null;if(x!==null)Nt=x.getViewport(mt);else{const Qt=y.getViewSubImage(g,mt);Nt=Qt.viewport,xt===0&&(e.setRenderTargetTextures(N,Qt.colorTexture,Qt.depthStencilTexture),e.setRenderTarget(N))}let ot=G[xt];ot===void 0&&(ot=new si,ot.layers.enable(xt),ot.viewport=new nn,G[xt]=ot),ot.matrix.fromArray(mt.transform.matrix),ot.matrix.decompose(ot.position,ot.quaternion,ot.scale),ot.projectionMatrix.fromArray(mt.projectionMatrix),ot.projectionMatrixInverse.copy(ot.projectionMatrix).invert(),ot.viewport.set(Nt.x,Nt.y,Nt.width,Nt.height),xt===0&&(te.matrix.copy(ot.matrix),te.matrix.decompose(te.position,te.quaternion,te.scale)),ke===!0&&te.cameras.push(ot)}const Ke=l.enabledFeatures;if(Ke&&Ke.includes("depth-sensing")&&l.depthUsage=="gpu-optimized"&&w){y=s.getBinding();const xt=y.getDepthInformation(Te[0]);xt&&xt.isValid&&xt.texture&&b.init(xt,l.renderState)}if(Ke&&Ke.includes("camera-access")&&w){e.state.unbindTexture(),y=s.getBinding();for(let xt=0;xt<Te.length;xt++){const mt=Te[xt].camera;if(mt){let Nt=S[mt];Nt||(Nt=new Mv,S[mt]=Nt);const ot=y.getCameraImage(mt);Nt.sourceTexture=ot}}}}for(let Te=0;Te<V.length;Te++){const ke=H[Te],Ke=V[Te];ke!==null&&Ke!==void 0&&Ke.update(ke,xe,h||d)}ve&&ve(ie,xe),xe.detectedPlanes&&s.dispatchEvent({type:"planesdetected",data:xe}),E=null}const Fe=new wv;Fe.setAnimationLoop(Re),this.setAnimationLoop=function(ie){ve=ie},this.dispose=function(){}}}const Ds=new ji,uA=new Jt;function fA(o,e){function i(b,S){b.matrixAutoUpdate===!0&&b.updateMatrix(),S.value.copy(b.matrix)}function s(b,S){S.color.getRGB(b.fogColor.value,Ev(o)),S.isFog?(b.fogNear.value=S.near,b.fogFar.value=S.far):S.isFogExp2&&(b.fogDensity.value=S.density)}function l(b,S,C,U,N){S.isMeshBasicMaterial?c(b,S):S.isMeshLambertMaterial?(c(b,S),S.envMap&&(b.envMapIntensity.value=S.envMapIntensity)):S.isMeshToonMaterial?(c(b,S),y(b,S)):S.isMeshPhongMaterial?(c(b,S),v(b,S),S.envMap&&(b.envMapIntensity.value=S.envMapIntensity)):S.isMeshStandardMaterial?(c(b,S),g(b,S),S.isMeshPhysicalMaterial&&x(b,S,N)):S.isMeshMatcapMaterial?(c(b,S),E(b,S)):S.isMeshDepthMaterial?c(b,S):S.isMeshDistanceMaterial?(c(b,S),w(b,S)):S.isMeshNormalMaterial?c(b,S):S.isLineBasicMaterial?(d(b,S),S.isLineDashedMaterial&&p(b,S)):S.isPointsMaterial?m(b,S,C,U):S.isSpriteMaterial?h(b,S):S.isShadowMaterial?(b.color.value.copy(S.color),b.opacity.value=S.opacity):S.isShaderMaterial&&(S.uniformsNeedUpdate=!1)}function c(b,S){b.opacity.value=S.opacity,S.color&&b.diffuse.value.copy(S.color),S.emissive&&b.emissive.value.copy(S.emissive).multiplyScalar(S.emissiveIntensity),S.map&&(b.map.value=S.map,i(S.map,b.mapTransform)),S.alphaMap&&(b.alphaMap.value=S.alphaMap,i(S.alphaMap,b.alphaMapTransform)),S.bumpMap&&(b.bumpMap.value=S.bumpMap,i(S.bumpMap,b.bumpMapTransform),b.bumpScale.value=S.bumpScale,S.side===qn&&(b.bumpScale.value*=-1)),S.normalMap&&(b.normalMap.value=S.normalMap,i(S.normalMap,b.normalMapTransform),b.normalScale.value.copy(S.normalScale),S.side===qn&&b.normalScale.value.negate()),S.displacementMap&&(b.displacementMap.value=S.displacementMap,i(S.displacementMap,b.displacementMapTransform),b.displacementScale.value=S.displacementScale,b.displacementBias.value=S.displacementBias),S.emissiveMap&&(b.emissiveMap.value=S.emissiveMap,i(S.emissiveMap,b.emissiveMapTransform)),S.specularMap&&(b.specularMap.value=S.specularMap,i(S.specularMap,b.specularMapTransform)),S.alphaTest>0&&(b.alphaTest.value=S.alphaTest);const C=e.get(S),U=C.envMap,N=C.envMapRotation;U&&(b.envMap.value=U,Ds.copy(N),Ds.x*=-1,Ds.y*=-1,Ds.z*=-1,U.isCubeTexture&&U.isRenderTargetTexture===!1&&(Ds.y*=-1,Ds.z*=-1),b.envMapRotation.value.setFromMatrix4(uA.makeRotationFromEuler(Ds)),b.flipEnvMap.value=U.isCubeTexture&&U.isRenderTargetTexture===!1?-1:1,b.reflectivity.value=S.reflectivity,b.ior.value=S.ior,b.refractionRatio.value=S.refractionRatio),S.lightMap&&(b.lightMap.value=S.lightMap,b.lightMapIntensity.value=S.lightMapIntensity,i(S.lightMap,b.lightMapTransform)),S.aoMap&&(b.aoMap.value=S.aoMap,b.aoMapIntensity.value=S.aoMapIntensity,i(S.aoMap,b.aoMapTransform))}function d(b,S){b.diffuse.value.copy(S.color),b.opacity.value=S.opacity,S.map&&(b.map.value=S.map,i(S.map,b.mapTransform))}function p(b,S){b.dashSize.value=S.dashSize,b.totalSize.value=S.dashSize+S.gapSize,b.scale.value=S.scale}function m(b,S,C,U){b.diffuse.value.copy(S.color),b.opacity.value=S.opacity,b.size.value=S.size*C,b.scale.value=U*.5,S.map&&(b.map.value=S.map,i(S.map,b.uvTransform)),S.alphaMap&&(b.alphaMap.value=S.alphaMap,i(S.alphaMap,b.alphaMapTransform)),S.alphaTest>0&&(b.alphaTest.value=S.alphaTest)}function h(b,S){b.diffuse.value.copy(S.color),b.opacity.value=S.opacity,b.rotation.value=S.rotation,S.map&&(b.map.value=S.map,i(S.map,b.mapTransform)),S.alphaMap&&(b.alphaMap.value=S.alphaMap,i(S.alphaMap,b.alphaMapTransform)),S.alphaTest>0&&(b.alphaTest.value=S.alphaTest)}function v(b,S){b.specular.value.copy(S.specular),b.shininess.value=Math.max(S.shininess,1e-4)}function y(b,S){S.gradientMap&&(b.gradientMap.value=S.gradientMap)}function g(b,S){b.metalness.value=S.metalness,S.metalnessMap&&(b.metalnessMap.value=S.metalnessMap,i(S.metalnessMap,b.metalnessMapTransform)),b.roughness.value=S.roughness,S.roughnessMap&&(b.roughnessMap.value=S.roughnessMap,i(S.roughnessMap,b.roughnessMapTransform)),S.envMap&&(b.envMapIntensity.value=S.envMapIntensity)}function x(b,S,C){b.ior.value=S.ior,S.sheen>0&&(b.sheenColor.value.copy(S.sheenColor).multiplyScalar(S.sheen),b.sheenRoughness.value=S.sheenRoughness,S.sheenColorMap&&(b.sheenColorMap.value=S.sheenColorMap,i(S.sheenColorMap,b.sheenColorMapTransform)),S.sheenRoughnessMap&&(b.sheenRoughnessMap.value=S.sheenRoughnessMap,i(S.sheenRoughnessMap,b.sheenRoughnessMapTransform))),S.clearcoat>0&&(b.clearcoat.value=S.clearcoat,b.clearcoatRoughness.value=S.clearcoatRoughness,S.clearcoatMap&&(b.clearcoatMap.value=S.clearcoatMap,i(S.clearcoatMap,b.clearcoatMapTransform)),S.clearcoatRoughnessMap&&(b.clearcoatRoughnessMap.value=S.clearcoatRoughnessMap,i(S.clearcoatRoughnessMap,b.clearcoatRoughnessMapTransform)),S.clearcoatNormalMap&&(b.clearcoatNormalMap.value=S.clearcoatNormalMap,i(S.clearcoatNormalMap,b.clearcoatNormalMapTransform),b.clearcoatNormalScale.value.copy(S.clearcoatNormalScale),S.side===qn&&b.clearcoatNormalScale.value.negate())),S.dispersion>0&&(b.dispersion.value=S.dispersion),S.iridescence>0&&(b.iridescence.value=S.iridescence,b.iridescenceIOR.value=S.iridescenceIOR,b.iridescenceThicknessMinimum.value=S.iridescenceThicknessRange[0],b.iridescenceThicknessMaximum.value=S.iridescenceThicknessRange[1],S.iridescenceMap&&(b.iridescenceMap.value=S.iridescenceMap,i(S.iridescenceMap,b.iridescenceMapTransform)),S.iridescenceThicknessMap&&(b.iridescenceThicknessMap.value=S.iridescenceThicknessMap,i(S.iridescenceThicknessMap,b.iridescenceThicknessMapTransform))),S.transmission>0&&(b.transmission.value=S.transmission,b.transmissionSamplerMap.value=C.texture,b.transmissionSamplerSize.value.set(C.width,C.height),S.transmissionMap&&(b.transmissionMap.value=S.transmissionMap,i(S.transmissionMap,b.transmissionMapTransform)),b.thickness.value=S.thickness,S.thicknessMap&&(b.thicknessMap.value=S.thicknessMap,i(S.thicknessMap,b.thicknessMapTransform)),b.attenuationDistance.value=S.attenuationDistance,b.attenuationColor.value.copy(S.attenuationColor)),S.anisotropy>0&&(b.anisotropyVector.value.set(S.anisotropy*Math.cos(S.anisotropyRotation),S.anisotropy*Math.sin(S.anisotropyRotation)),S.anisotropyMap&&(b.anisotropyMap.value=S.anisotropyMap,i(S.anisotropyMap,b.anisotropyMapTransform))),b.specularIntensity.value=S.specularIntensity,b.specularColor.value.copy(S.specularColor),S.specularColorMap&&(b.specularColorMap.value=S.specularColorMap,i(S.specularColorMap,b.specularColorMapTransform)),S.specularIntensityMap&&(b.specularIntensityMap.value=S.specularIntensityMap,i(S.specularIntensityMap,b.specularIntensityMapTransform))}function E(b,S){S.matcap&&(b.matcap.value=S.matcap)}function w(b,S){const C=e.get(S).light;b.referencePosition.value.setFromMatrixPosition(C.matrixWorld),b.nearDistance.value=C.shadow.camera.near,b.farDistance.value=C.shadow.camera.far}return{refreshFogUniforms:s,refreshMaterialUniforms:l}}function dA(o,e,i,s){let l={},c={},d=[];const p=o.getParameter(o.MAX_UNIFORM_BUFFER_BINDINGS);function m(C,U){const N=U.program;s.uniformBlockBinding(C,N)}function h(C,U){let N=l[C.id];N===void 0&&(E(C),N=v(C),l[C.id]=N,C.addEventListener("dispose",b));const V=U.program;s.updateUBOMapping(C,V);const H=e.render.frame;c[C.id]!==H&&(g(C),c[C.id]=H)}function v(C){const U=y();C.__bindingPointIndex=U;const N=o.createBuffer(),V=C.__size,H=C.usage;return o.bindBuffer(o.UNIFORM_BUFFER,N),o.bufferData(o.UNIFORM_BUFFER,V,H),o.bindBuffer(o.UNIFORM_BUFFER,null),o.bindBufferBase(o.UNIFORM_BUFFER,U,N),N}function y(){for(let C=0;C<p;C++)if(d.indexOf(C)===-1)return d.push(C),C;return Dt("WebGLRenderer: Maximum number of simultaneously usable uniforms groups reached."),0}function g(C){const U=l[C.id],N=C.uniforms,V=C.__cache;o.bindBuffer(o.UNIFORM_BUFFER,U);for(let H=0,F=N.length;H<F;H++){const T=Array.isArray(N[H])?N[H]:[N[H]];for(let D=0,le=T.length;D<le;D++){const G=T[D];if(x(G,H,D,V)===!0){const te=G.__offset,se=Array.isArray(G.value)?G.value:[G.value];let ue=0;for(let ee=0;ee<se.length;ee++){const P=se[ee],z=w(P);typeof P=="number"||typeof P=="boolean"?(G.__data[0]=P,o.bufferSubData(o.UNIFORM_BUFFER,te+ue,G.__data)):P.isMatrix3?(G.__data[0]=P.elements[0],G.__data[1]=P.elements[1],G.__data[2]=P.elements[2],G.__data[3]=0,G.__data[4]=P.elements[3],G.__data[5]=P.elements[4],G.__data[6]=P.elements[5],G.__data[7]=0,G.__data[8]=P.elements[6],G.__data[9]=P.elements[7],G.__data[10]=P.elements[8],G.__data[11]=0):(P.toArray(G.__data,ue),ue+=z.storage/Float32Array.BYTES_PER_ELEMENT)}o.bufferSubData(o.UNIFORM_BUFFER,te,G.__data)}}}o.bindBuffer(o.UNIFORM_BUFFER,null)}function x(C,U,N,V){const H=C.value,F=U+"_"+N;if(V[F]===void 0)return typeof H=="number"||typeof H=="boolean"?V[F]=H:V[F]=H.clone(),!0;{const T=V[F];if(typeof H=="number"||typeof H=="boolean"){if(T!==H)return V[F]=H,!0}else if(T.equals(H)===!1)return T.copy(H),!0}return!1}function E(C){const U=C.uniforms;let N=0;const V=16;for(let F=0,T=U.length;F<T;F++){const D=Array.isArray(U[F])?U[F]:[U[F]];for(let le=0,G=D.length;le<G;le++){const te=D[le],se=Array.isArray(te.value)?te.value:[te.value];for(let ue=0,ee=se.length;ue<ee;ue++){const P=se[ue],z=w(P),ce=N%V,pe=ce%z.boundary,Ee=ce+pe;N+=pe,Ee!==0&&V-Ee<z.storage&&(N+=V-Ee),te.__data=new Float32Array(z.storage/Float32Array.BYTES_PER_ELEMENT),te.__offset=N,N+=z.storage}}}const H=N%V;return H>0&&(N+=V-H),C.__size=N,C.__cache={},this}function w(C){const U={boundary:0,storage:0};return typeof C=="number"||typeof C=="boolean"?(U.boundary=4,U.storage=4):C.isVector2?(U.boundary=8,U.storage=8):C.isVector3||C.isColor?(U.boundary=16,U.storage=12):C.isVector4?(U.boundary=16,U.storage=16):C.isMatrix3?(U.boundary=48,U.storage=48):C.isMatrix4?(U.boundary=64,U.storage=64):C.isTexture?at("WebGLRenderer: Texture samplers can not be part of an uniforms group."):at("WebGLRenderer: Unsupported uniform value type.",C),U}function b(C){const U=C.target;U.removeEventListener("dispose",b);const N=d.indexOf(U.__bindingPointIndex);d.splice(N,1),o.deleteBuffer(l[U.id]),delete l[U.id],delete c[U.id]}function S(){for(const C in l)o.deleteBuffer(l[C]);d=[],l={},c={}}return{bind:m,update:h,dispose:S}}const hA=new Uint16Array([12469,15057,12620,14925,13266,14620,13807,14376,14323,13990,14545,13625,14713,13328,14840,12882,14931,12528,14996,12233,15039,11829,15066,11525,15080,11295,15085,10976,15082,10705,15073,10495,13880,14564,13898,14542,13977,14430,14158,14124,14393,13732,14556,13410,14702,12996,14814,12596,14891,12291,14937,11834,14957,11489,14958,11194,14943,10803,14921,10506,14893,10278,14858,9960,14484,14039,14487,14025,14499,13941,14524,13740,14574,13468,14654,13106,14743,12678,14818,12344,14867,11893,14889,11509,14893,11180,14881,10751,14852,10428,14812,10128,14765,9754,14712,9466,14764,13480,14764,13475,14766,13440,14766,13347,14769,13070,14786,12713,14816,12387,14844,11957,14860,11549,14868,11215,14855,10751,14825,10403,14782,10044,14729,9651,14666,9352,14599,9029,14967,12835,14966,12831,14963,12804,14954,12723,14936,12564,14917,12347,14900,11958,14886,11569,14878,11247,14859,10765,14828,10401,14784,10011,14727,9600,14660,9289,14586,8893,14508,8533,15111,12234,15110,12234,15104,12216,15092,12156,15067,12010,15028,11776,14981,11500,14942,11205,14902,10752,14861,10393,14812,9991,14752,9570,14682,9252,14603,8808,14519,8445,14431,8145,15209,11449,15208,11451,15202,11451,15190,11438,15163,11384,15117,11274,15055,10979,14994,10648,14932,10343,14871,9936,14803,9532,14729,9218,14645,8742,14556,8381,14461,8020,14365,7603,15273,10603,15272,10607,15267,10619,15256,10631,15231,10614,15182,10535,15118,10389,15042,10167,14963,9787,14883,9447,14800,9115,14710,8665,14615,8318,14514,7911,14411,7507,14279,7198,15314,9675,15313,9683,15309,9712,15298,9759,15277,9797,15229,9773,15166,9668,15084,9487,14995,9274,14898,8910,14800,8539,14697,8234,14590,7790,14479,7409,14367,7067,14178,6621,15337,8619,15337,8631,15333,8677,15325,8769,15305,8871,15264,8940,15202,8909,15119,8775,15022,8565,14916,8328,14804,8009,14688,7614,14569,7287,14448,6888,14321,6483,14088,6171,15350,7402,15350,7419,15347,7480,15340,7613,15322,7804,15287,7973,15229,8057,15148,8012,15046,7846,14933,7611,14810,7357,14682,7069,14552,6656,14421,6316,14251,5948,14007,5528,15356,5942,15356,5977,15353,6119,15348,6294,15332,6551,15302,6824,15249,7044,15171,7122,15070,7050,14949,6861,14818,6611,14679,6349,14538,6067,14398,5651,14189,5311,13935,4958,15359,4123,15359,4153,15356,4296,15353,4646,15338,5160,15311,5508,15263,5829,15188,6042,15088,6094,14966,6001,14826,5796,14678,5543,14527,5287,14377,4985,14133,4586,13869,4257,15360,1563,15360,1642,15358,2076,15354,2636,15341,3350,15317,4019,15273,4429,15203,4732,15105,4911,14981,4932,14836,4818,14679,4621,14517,4386,14359,4156,14083,3795,13808,3437,15360,122,15360,137,15358,285,15355,636,15344,1274,15322,2177,15281,2765,15215,3223,15120,3451,14995,3569,14846,3567,14681,3466,14511,3305,14344,3121,14037,2800,13753,2467,15360,0,15360,1,15359,21,15355,89,15346,253,15325,479,15287,796,15225,1148,15133,1492,15008,1749,14856,1882,14685,1886,14506,1783,14324,1608,13996,1398,13702,1183]);let zi=null;function pA(){return zi===null&&(zi=new tb(hA,16,16,Gr,ba),zi.name="DFG_LUT",zi.minFilter=Dn,zi.magFilter=Dn,zi.wrapS=xa,zi.wrapT=xa,zi.generateMipmaps=!1,zi.needsUpdate=!0),zi}class mA{constructor(e={}){const{canvas:i=NS(),context:s=null,depth:l=!0,stencil:c=!1,alpha:d=!1,antialias:p=!1,premultipliedAlpha:m=!0,preserveDrawingBuffer:h=!1,powerPreference:v="default",failIfMajorPerformanceCaveat:y=!1,reversedDepthBuffer:g=!1,outputBufferType:x=ri}=e;this.isWebGLRenderer=!0;let E;if(s!==null){if(typeof WebGLRenderingContext<"u"&&s instanceof WebGLRenderingContext)throw new Error("THREE.WebGLRenderer: WebGL 1 is not supported since r163.");E=s.getContextAttributes().alpha}else E=d;const w=x,b=new Set([Zh,Yh,qh]),S=new Set([ri,Xi,qo,Yo,jh,Wh]),C=new Uint32Array(4),U=new Int32Array(4);let N=null,V=null;const H=[],F=[];let T=null;this.domElement=i,this.debug={checkShaderErrors:!0,onShaderError:null},this.autoClear=!0,this.autoClearColor=!0,this.autoClearDepth=!0,this.autoClearStencil=!0,this.sortObjects=!0,this.clippingPlanes=[],this.localClippingEnabled=!1,this.toneMapping=Vi,this.toneMappingExposure=1,this.transmissionResolutionScale=1;const D=this;let le=!1;this._outputColorSpace=gi;let G=0,te=0,se=null,ue=-1,ee=null;const P=new nn,z=new nn;let ce=null;const pe=new At(0);let Ee=0,I=i.width,Y=i.height,ve=1,Re=null,Fe=null;const ie=new nn(0,0,I,Y),xe=new nn(0,0,I,Y);let Te=!1;const ke=new tp;let Ke=!1,$e=!1;const $t=new Jt,xt=new K,mt=new nn,Nt={background:null,fog:null,environment:null,overrideMaterial:null,isScene:!0};let ot=!1;function Qt(){return se===null?ve:1}let k=s;function qt(R,j){return i.getContext(R,j)}try{const R={alpha:!0,depth:l,stencil:c,antialias:p,premultipliedAlpha:m,preserveDrawingBuffer:h,powerPreference:v,failIfMajorPerformanceCaveat:y};if("setAttribute"in i&&i.setAttribute("data-engine",`three.js r${kh}`),i.addEventListener("webglcontextlost",Be,!1),i.addEventListener("webglcontextrestored",nt,!1),i.addEventListener("webglcontextcreationerror",Pt,!1),k===null){const j="webgl2";if(k=qt(j,R),k===null)throw qt(j)?new Error("Error creating WebGL context with your selected attributes."):new Error("Error creating WebGL context.")}}catch(R){throw Dt("WebGLRenderer: "+R.message),R}let Mt,Lt,We,L,M,q,me,ye,de,Xe,Ce,Ze,et,Me,Se,Oe,Le,Pe,ut,W,we,Ae,Ie;function be(){Mt=new mT(k),Mt.init(),we=new sA(k,Mt),Lt=new oT(k,Mt,e,we),We=new iA(k,Mt),Lt.reversedDepthBuffer&&g&&We.buffers.depth.setReversed(!0),L=new vT(k),M=new k1,q=new aA(k,Mt,We,M,Lt,we,L),me=new pT(D),ye=new Mb(k),Ae=new sT(k,ye),de=new gT(k,ye,L,Ae),Xe=new yT(k,de,ye,Ae,L),Pe=new xT(k,Lt,q),Se=new lT(M),Ce=new V1(D,me,Mt,Lt,Ae,Se),Ze=new fA(D,M),et=new j1,Me=new Q1(Mt),Le=new aT(D,me,We,Xe,E,m),Oe=new nA(D,Xe,Lt),Ie=new dA(k,L,Lt,We),ut=new rT(k,Mt,L),W=new _T(k,Mt,L),L.programs=Ce.programs,D.capabilities=Lt,D.extensions=Mt,D.properties=M,D.renderLists=et,D.shadowMap=Oe,D.state=We,D.info=L}be(),w!==ri&&(T=new bT(w,i.width,i.height,l,c));const fe=new cA(D,k);this.xr=fe,this.getContext=function(){return k},this.getContextAttributes=function(){return k.getContextAttributes()},this.forceContextLoss=function(){const R=Mt.get("WEBGL_lose_context");R&&R.loseContext()},this.forceContextRestore=function(){const R=Mt.get("WEBGL_lose_context");R&&R.restoreContext()},this.getPixelRatio=function(){return ve},this.setPixelRatio=function(R){R!==void 0&&(ve=R,this.setSize(I,Y,!1))},this.getSize=function(R){return R.set(I,Y)},this.setSize=function(R,j,re=!0){if(fe.isPresenting){at("WebGLRenderer: Can't change size while VR device is presenting.");return}I=R,Y=j,i.width=Math.floor(R*ve),i.height=Math.floor(j*ve),re===!0&&(i.style.width=R+"px",i.style.height=j+"px"),T!==null&&T.setSize(i.width,i.height),this.setViewport(0,0,R,j)},this.getDrawingBufferSize=function(R){return R.set(I*ve,Y*ve).floor()},this.setDrawingBufferSize=function(R,j,re){I=R,Y=j,ve=re,i.width=Math.floor(R*re),i.height=Math.floor(j*re),this.setViewport(0,0,R,j)},this.setEffects=function(R){if(w===ri){console.error("THREE.WebGLRenderer: setEffects() requires outputBufferType set to HalfFloatType or FloatType.");return}if(R){for(let j=0;j<R.length;j++)if(R[j].isOutputPass===!0){console.warn("THREE.WebGLRenderer: OutputPass is not needed in setEffects(). Tone mapping and color space conversion are applied automatically.");break}}T.setEffects(R||[])},this.getCurrentViewport=function(R){return R.copy(P)},this.getViewport=function(R){return R.copy(ie)},this.setViewport=function(R,j,re,ne){R.isVector4?ie.set(R.x,R.y,R.z,R.w):ie.set(R,j,re,ne),We.viewport(P.copy(ie).multiplyScalar(ve).round())},this.getScissor=function(R){return R.copy(xe)},this.setScissor=function(R,j,re,ne){R.isVector4?xe.set(R.x,R.y,R.z,R.w):xe.set(R,j,re,ne),We.scissor(z.copy(xe).multiplyScalar(ve).round())},this.getScissorTest=function(){return Te},this.setScissorTest=function(R){We.setScissorTest(Te=R)},this.setOpaqueSort=function(R){Re=R},this.setTransparentSort=function(R){Fe=R},this.getClearColor=function(R){return R.copy(Le.getClearColor())},this.setClearColor=function(){Le.setClearColor(...arguments)},this.getClearAlpha=function(){return Le.getClearAlpha()},this.setClearAlpha=function(){Le.setClearAlpha(...arguments)},this.clear=function(R=!0,j=!0,re=!0){let ne=0;if(R){let Q=!1;if(se!==null){const De=se.texture.format;Q=b.has(De)}if(Q){const De=se.texture.type,ze=S.has(De),Ne=Le.getClearColor(),je=Le.getClearAlpha(),Ye=Ne.r,tt=Ne.g,st=Ne.b;ze?(C[0]=Ye,C[1]=tt,C[2]=st,C[3]=je,k.clearBufferuiv(k.COLOR,0,C)):(U[0]=Ye,U[1]=tt,U[2]=st,U[3]=je,k.clearBufferiv(k.COLOR,0,U))}else ne|=k.COLOR_BUFFER_BIT}j&&(ne|=k.DEPTH_BUFFER_BIT),re&&(ne|=k.STENCIL_BUFFER_BIT,this.state.buffers.stencil.setMask(4294967295)),ne!==0&&k.clear(ne)},this.clearColor=function(){this.clear(!0,!1,!1)},this.clearDepth=function(){this.clear(!1,!0,!1)},this.clearStencil=function(){this.clear(!1,!1,!0)},this.dispose=function(){i.removeEventListener("webglcontextlost",Be,!1),i.removeEventListener("webglcontextrestored",nt,!1),i.removeEventListener("webglcontextcreationerror",Pt,!1),Le.dispose(),et.dispose(),Me.dispose(),M.dispose(),me.dispose(),Xe.dispose(),Ae.dispose(),Ie.dispose(),Ce.dispose(),fe.dispose(),fe.removeEventListener("sessionstart",Bs),fe.removeEventListener("sessionend",Hs),Ui.stop()};function Be(R){R.preventDefault(),$0("WebGLRenderer: Context Lost."),le=!0}function nt(){$0("WebGLRenderer: Context Restored."),le=!1;const R=L.autoReset,j=Oe.enabled,re=Oe.autoUpdate,ne=Oe.needsUpdate,Q=Oe.type;be(),L.autoReset=R,Oe.enabled=j,Oe.autoUpdate=re,Oe.needsUpdate=ne,Oe.type=Q}function Pt(R){Dt("WebGLRenderer: A WebGL context could not be created. Reason: ",R.statusMessage)}function Et(R){const j=R.target;j.removeEventListener("dispose",Et),Nn(j)}function Nn(R){xi(R),M.remove(R)}function xi(R){const j=M.get(R).programs;j!==void 0&&(j.forEach(function(re){Ce.releaseProgram(re)}),R.isShaderMaterial&&Ce.releaseShaderCache(R))}this.renderBufferDirect=function(R,j,re,ne,Q,De){j===null&&(j=Nt);const ze=Q.isMesh&&Q.matrixWorld.determinant()<0,Ne=il(R,j,re,ne,Q);We.setMaterial(ne,ze);let je=re.index,Ye=1;if(ne.wireframe===!0){if(je=de.getWireframeAttribute(re),je===void 0)return;Ye=2}const tt=re.drawRange,st=re.attributes.position;let He=tt.start*Ye,ft=(tt.start+tt.count)*Ye;De!==null&&(He=Math.max(He,De.start*Ye),ft=Math.min(ft,(De.start+De.count)*Ye)),je!==null?(He=Math.max(He,0),ft=Math.min(ft,je.count)):st!=null&&(He=Math.max(He,0),ft=Math.min(ft,st.count));const Yt=ft-He;if(Yt<0||Yt===1/0)return;Ae.setup(Q,ne,Ne,re,je);let Zt,Rt=ut;if(je!==null&&(Zt=ye.get(je),Rt=W,Rt.setIndex(Zt)),Q.isMesh)ne.wireframe===!0?(We.setLineWidth(ne.wireframeLinewidth*Qt()),Rt.setMode(k.LINES)):Rt.setMode(k.TRIANGLES);else if(Q.isLine){let mn=ne.linewidth;mn===void 0&&(mn=1),We.setLineWidth(mn*Qt()),Q.isLineSegments?Rt.setMode(k.LINES):Q.isLineLoop?Rt.setMode(k.LINE_LOOP):Rt.setMode(k.LINE_STRIP)}else Q.isPoints?Rt.setMode(k.POINTS):Q.isSprite&&Rt.setMode(k.TRIANGLES);if(Q.isBatchedMesh)if(Q._multiDrawInstances!==null)Kc("WebGLRenderer: renderMultiDrawInstances has been deprecated and will be removed in r184. Append to renderMultiDraw arguments and use indirection."),Rt.renderMultiDrawInstances(Q._multiDrawStarts,Q._multiDrawCounts,Q._multiDrawCount,Q._multiDrawInstances);else if(Mt.get("WEBGL_multi_draw"))Rt.renderMultiDraw(Q._multiDrawStarts,Q._multiDrawCounts,Q._multiDrawCount);else{const mn=Q._multiDrawStarts,Ve=Q._multiDrawCounts,Un=Q._multiDrawCount,it=je?ye.get(je).bytesPerElement:1,Ln=M.get(ne).currentProgram.getUniforms();for(let Yn=0;Yn<Un;Yn++)Ln.setValue(k,"_gl_DrawID",Yn),Rt.render(mn[Yn]/it,Ve[Yn])}else if(Q.isInstancedMesh)Rt.renderInstances(He,Yt,Q.count);else if(re.isInstancedBufferGeometry){const mn=re._maxInstanceCount!==void 0?re._maxInstanceCount:1/0,Ve=Math.min(re.instanceCount,mn);Rt.renderInstances(He,Yt,Ve)}else Rt.render(He,Yt)};function Wr(R,j,re){R.transparent===!0&&R.side===va&&R.forceSinglePass===!1?(R.side=qn,R.needsUpdate=!0,Ea(R,j,re),R.side=ss,R.needsUpdate=!0,Ea(R,j,re),R.side=va):Ea(R,j,re)}this.compile=function(R,j,re=null){re===null&&(re=R),V=Me.get(re),V.init(j),F.push(V),re.traverseVisible(function(Q){Q.isLight&&Q.layers.test(j.layers)&&(V.pushLight(Q),Q.castShadow&&V.pushShadow(Q))}),R!==re&&R.traverseVisible(function(Q){Q.isLight&&Q.layers.test(j.layers)&&(V.pushLight(Q),Q.castShadow&&V.pushShadow(Q))}),V.setupLights();const ne=new Set;return R.traverse(function(Q){if(!(Q.isMesh||Q.isPoints||Q.isLine||Q.isSprite))return;const De=Q.material;if(De)if(Array.isArray(De))for(let ze=0;ze<De.length;ze++){const Ne=De[ze];Wr(Ne,re,Q),ne.add(Ne)}else Wr(De,re,Q),ne.add(De)}),V=F.pop(),ne},this.compileAsync=function(R,j,re=null){const ne=this.compile(R,j,re);return new Promise(Q=>{function De(){if(ne.forEach(function(ze){M.get(ze).currentProgram.isReady()&&ne.delete(ze)}),ne.size===0){Q(R);return}setTimeout(De,10)}Mt.get("KHR_parallel_shader_compile")!==null?De():setTimeout(De,10)})};let zs=null;function el(R){zs&&zs(R)}function Bs(){Ui.stop()}function Hs(){Ui.start()}const Ui=new wv;Ui.setAnimationLoop(el),typeof self<"u"&&Ui.setContext(self),this.setAnimationLoop=function(R){zs=R,fe.setAnimationLoop(R),R===null?Ui.stop():Ui.start()},fe.addEventListener("sessionstart",Bs),fe.addEventListener("sessionend",Hs),this.render=function(R,j){if(j!==void 0&&j.isCamera!==!0){Dt("WebGLRenderer.render: camera is not an instance of THREE.Camera.");return}if(le===!0)return;const re=fe.enabled===!0&&fe.isPresenting===!0,ne=T!==null&&(se===null||re)&&T.begin(D,se);if(R.matrixWorldAutoUpdate===!0&&R.updateMatrixWorld(),j.parent===null&&j.matrixWorldAutoUpdate===!0&&j.updateMatrixWorld(),fe.enabled===!0&&fe.isPresenting===!0&&(T===null||T.isCompositing()===!1)&&(fe.cameraAutoUpdate===!0&&fe.updateCamera(j),j=fe.getCamera()),R.isScene===!0&&R.onBeforeRender(D,R,j,se),V=Me.get(R,F.length),V.init(j),F.push(V),$t.multiplyMatrices(j.projectionMatrix,j.matrixWorldInverse),ke.setFromProjectionMatrix($t,Gi,j.reversedDepth),$e=this.localClippingEnabled,Ke=Se.init(this.clippingPlanes,$e),N=et.get(R,H.length),N.init(),H.push(N),fe.enabled===!0&&fe.isPresenting===!0){const ze=D.xr.getDepthSensingMesh();ze!==null&&Gs(ze,j,-1/0,D.sortObjects)}Gs(R,j,0,D.sortObjects),N.finish(),D.sortObjects===!0&&N.sort(Re,Fe),ot=fe.enabled===!1||fe.isPresenting===!1||fe.hasDepthSensing()===!1,ot&&Le.addToRenderList(N,R),this.info.render.frame++,Ke===!0&&Se.beginShadows();const Q=V.state.shadowsArray;if(Oe.render(Q,R,j),Ke===!0&&Se.endShadows(),this.info.autoReset===!0&&this.info.reset(),(ne&&T.hasRenderPass())===!1){const ze=N.opaque,Ne=N.transmissive;if(V.setupLights(),j.isArrayCamera){const je=j.cameras;if(Ne.length>0)for(let Ye=0,tt=je.length;Ye<tt;Ye++){const st=je[Ye];rn(ze,Ne,R,st)}ot&&Le.render(R);for(let Ye=0,tt=je.length;Ye<tt;Ye++){const st=je[Ye];yi(N,R,st,st.viewport)}}else Ne.length>0&&rn(ze,Ne,R,j),ot&&Le.render(R),yi(N,R,j)}se!==null&&te===0&&(q.updateMultisampleRenderTarget(se),q.updateRenderTargetMipmap(se)),ne&&T.end(D),R.isScene===!0&&R.onAfterRender(D,R,j),Ae.resetDefaultState(),ue=-1,ee=null,F.pop(),F.length>0?(V=F[F.length-1],Ke===!0&&Se.setGlobalState(D.clippingPlanes,V.state.camera)):V=null,H.pop(),H.length>0?N=H[H.length-1]:N=null};function Gs(R,j,re,ne){if(R.visible===!1)return;if(R.layers.test(j.layers)){if(R.isGroup)re=R.renderOrder;else if(R.isLOD)R.autoUpdate===!0&&R.update(j);else if(R.isLight)V.pushLight(R),R.castShadow&&V.pushShadow(R);else if(R.isSprite){if(!R.frustumCulled||ke.intersectsSprite(R)){ne&&mt.setFromMatrixPosition(R.matrixWorld).applyMatrix4($t);const ze=Xe.update(R),Ne=R.material;Ne.visible&&N.push(R,ze,Ne,re,mt.z,null)}}else if((R.isMesh||R.isLine||R.isPoints)&&(!R.frustumCulled||ke.intersectsObject(R))){const ze=Xe.update(R),Ne=R.material;if(ne&&(R.boundingSphere!==void 0?(R.boundingSphere===null&&R.computeBoundingSphere(),mt.copy(R.boundingSphere.center)):(ze.boundingSphere===null&&ze.computeBoundingSphere(),mt.copy(ze.boundingSphere.center)),mt.applyMatrix4(R.matrixWorld).applyMatrix4($t)),Array.isArray(Ne)){const je=ze.groups;for(let Ye=0,tt=je.length;Ye<tt;Ye++){const st=je[Ye],He=Ne[st.materialIndex];He&&He.visible&&N.push(R,ze,He,re,mt.z,st)}}else Ne.visible&&N.push(R,ze,Ne,re,mt.z,null)}}const De=R.children;for(let ze=0,Ne=De.length;ze<Ne;ze++)Gs(De[ze],j,re,ne)}function yi(R,j,re,ne){const{opaque:Q,transmissive:De,transparent:ze}=R;V.setupLightsView(re),Ke===!0&&Se.setGlobalState(D.clippingPlanes,re),ne&&We.viewport(P.copy(ne)),Q.length>0&&pn(Q,j,re),De.length>0&&pn(De,j,re),ze.length>0&&pn(ze,j,re),We.buffers.depth.setTest(!0),We.buffers.depth.setMask(!0),We.buffers.color.setMask(!0),We.setPolygonOffset(!1)}function rn(R,j,re,ne){if((re.isScene===!0?re.overrideMaterial:null)!==null)return;if(V.state.transmissionRenderTarget[ne.id]===void 0){const He=Mt.has("EXT_color_buffer_half_float")||Mt.has("EXT_color_buffer_float");V.state.transmissionRenderTarget[ne.id]=new ki(1,1,{generateMipmaps:!0,type:He?ba:ri,minFilter:Os,samples:Math.max(4,Lt.samples),stencilBuffer:c,resolveDepthBuffer:!1,resolveStencilBuffer:!1,colorSpace:Tt.workingColorSpace})}const De=V.state.transmissionRenderTarget[ne.id],ze=ne.viewport||P;De.setSize(ze.z*D.transmissionResolutionScale,ze.w*D.transmissionResolutionScale);const Ne=D.getRenderTarget(),je=D.getActiveCubeFace(),Ye=D.getActiveMipmapLevel();D.setRenderTarget(De),D.getClearColor(pe),Ee=D.getClearAlpha(),Ee<1&&D.setClearColor(16777215,.5),D.clear(),ot&&Le.render(re);const tt=D.toneMapping;D.toneMapping=Vi;const st=ne.viewport;if(ne.viewport!==void 0&&(ne.viewport=void 0),V.setupLightsView(ne),Ke===!0&&Se.setGlobalState(D.clippingPlanes,ne),pn(R,re,ne),q.updateMultisampleRenderTarget(De),q.updateRenderTargetMipmap(De),Mt.has("WEBGL_multisampled_render_to_texture")===!1){let He=!1;for(let ft=0,Yt=j.length;ft<Yt;ft++){const Zt=j[ft],{object:Rt,geometry:mn,material:Ve,group:Un}=Zt;if(Ve.side===va&&Rt.layers.test(ne.layers)){const it=Ve.side;Ve.side=qn,Ve.needsUpdate=!0,Yi(Rt,re,ne,mn,Ve,Un),Ve.side=it,Ve.needsUpdate=!0,He=!0}}He===!0&&(q.updateMultisampleRenderTarget(De),q.updateRenderTargetMipmap(De))}D.setRenderTarget(Ne,je,Ye),D.setClearColor(pe,Ee),st!==void 0&&(ne.viewport=st),D.toneMapping=tt}function pn(R,j,re){const ne=j.isScene===!0?j.overrideMaterial:null;for(let Q=0,De=R.length;Q<De;Q++){const ze=R[Q],{object:Ne,geometry:je,group:Ye}=ze;let tt=ze.material;tt.allowOverride===!0&&ne!==null&&(tt=ne),Ne.layers.test(re.layers)&&Yi(Ne,j,re,je,tt,Ye)}}function Yi(R,j,re,ne,Q,De){R.onBeforeRender(D,j,re,ne,Q,De),R.modelViewMatrix.multiplyMatrices(re.matrixWorldInverse,R.matrixWorld),R.normalMatrix.getNormalMatrix(R.modelViewMatrix),Q.onBeforeRender(D,j,re,ne,R,De),Q.transparent===!0&&Q.side===va&&Q.forceSinglePass===!1?(Q.side=qn,Q.needsUpdate=!0,D.renderBufferDirect(re,j,ne,Q,R,De),Q.side=ss,Q.needsUpdate=!0,D.renderBufferDirect(re,j,ne,Q,R,De),Q.side=va):D.renderBufferDirect(re,j,ne,Q,R,De),R.onAfterRender(D,j,re,ne,Q,De)}function Ea(R,j,re){j.isScene!==!0&&(j=Nt);const ne=M.get(R),Q=V.state.lights,De=V.state.shadowsArray,ze=Q.state.version,Ne=Ce.getParameters(R,Q.state,De,j,re),je=Ce.getProgramCacheKey(Ne);let Ye=ne.programs;ne.environment=R.isMeshStandardMaterial||R.isMeshLambertMaterial||R.isMeshPhongMaterial?j.environment:null,ne.fog=j.fog;const tt=R.isMeshStandardMaterial||R.isMeshLambertMaterial&&!R.envMap||R.isMeshPhongMaterial&&!R.envMap;ne.envMap=me.get(R.envMap||ne.environment,tt),ne.envMapRotation=ne.environment!==null&&R.envMap===null?j.environmentRotation:R.envMapRotation,Ye===void 0&&(R.addEventListener("dispose",Et),Ye=new Map,ne.programs=Ye);let st=Ye.get(je);if(st!==void 0){if(ne.currentProgram===st&&ne.lightsStateVersion===ze)return nl(R,Ne),st}else Ne.uniforms=Ce.getUniforms(R),R.onBeforeCompile(Ne,D),st=Ce.acquireProgram(Ne,je),Ye.set(je,st),ne.uniforms=Ne.uniforms;const He=ne.uniforms;return(!R.isShaderMaterial&&!R.isRawShaderMaterial||R.clipping===!0)&&(He.clippingPlanes=Se.uniform),nl(R,Ne),ne.needsLights=qr(R),ne.lightsStateVersion=ze,ne.needsLights&&(He.ambientLightColor.value=Q.state.ambient,He.lightProbe.value=Q.state.probe,He.directionalLights.value=Q.state.directional,He.directionalLightShadows.value=Q.state.directionalShadow,He.spotLights.value=Q.state.spot,He.spotLightShadows.value=Q.state.spotShadow,He.rectAreaLights.value=Q.state.rectArea,He.ltc_1.value=Q.state.rectAreaLTC1,He.ltc_2.value=Q.state.rectAreaLTC2,He.pointLights.value=Q.state.point,He.pointLightShadows.value=Q.state.pointShadow,He.hemisphereLights.value=Q.state.hemi,He.directionalShadowMatrix.value=Q.state.directionalShadowMatrix,He.spotLightMatrix.value=Q.state.spotLightMatrix,He.spotLightMap.value=Q.state.spotLightMap,He.pointShadowMatrix.value=Q.state.pointShadowMatrix),ne.currentProgram=st,ne.uniformsList=null,st}function tl(R){if(R.uniformsList===null){const j=R.currentProgram.getUniforms();R.uniformsList=qc.seqWithValue(j.seq,R.uniforms)}return R.uniformsList}function nl(R,j){const re=M.get(R);re.outputColorSpace=j.outputColorSpace,re.batching=j.batching,re.batchingColor=j.batchingColor,re.instancing=j.instancing,re.instancingColor=j.instancingColor,re.instancingMorph=j.instancingMorph,re.skinning=j.skinning,re.morphTargets=j.morphTargets,re.morphNormals=j.morphNormals,re.morphColors=j.morphColors,re.morphTargetsCount=j.morphTargetsCount,re.numClippingPlanes=j.numClippingPlanes,re.numIntersection=j.numClipIntersection,re.vertexAlphas=j.vertexAlphas,re.vertexTangents=j.vertexTangents,re.toneMapping=j.toneMapping}function il(R,j,re,ne,Q){j.isScene!==!0&&(j=Nt),q.resetTextureUnits();const De=j.fog,ze=ne.isMeshStandardMaterial||ne.isMeshLambertMaterial||ne.isMeshPhongMaterial?j.environment:null,Ne=se===null?D.outputColorSpace:se.isXRRenderTarget===!0?se.texture.colorSpace:Vr,je=ne.isMeshStandardMaterial||ne.isMeshLambertMaterial&&!ne.envMap||ne.isMeshPhongMaterial&&!ne.envMap,Ye=me.get(ne.envMap||ze,je),tt=ne.vertexColors===!0&&!!re.attributes.color&&re.attributes.color.itemSize===4,st=!!re.attributes.tangent&&(!!ne.normalMap||ne.anisotropy>0),He=!!re.morphAttributes.position,ft=!!re.morphAttributes.normal,Yt=!!re.morphAttributes.color;let Zt=Vi;ne.toneMapped&&(se===null||se.isXRRenderTarget===!0)&&(Zt=D.toneMapping);const Rt=re.morphAttributes.position||re.morphAttributes.normal||re.morphAttributes.color,mn=Rt!==void 0?Rt.length:0,Ve=M.get(ne),Un=V.state.lights;if(Ke===!0&&($e===!0||R!==ee)){const ln=R===ee&&ne.id===ue;Se.setState(ne,R,ln)}let it=!1;ne.version===Ve.__version?(Ve.needsLights&&Ve.lightsStateVersion!==Un.state.version||Ve.outputColorSpace!==Ne||Q.isBatchedMesh&&Ve.batching===!1||!Q.isBatchedMesh&&Ve.batching===!0||Q.isBatchedMesh&&Ve.batchingColor===!0&&Q.colorTexture===null||Q.isBatchedMesh&&Ve.batchingColor===!1&&Q.colorTexture!==null||Q.isInstancedMesh&&Ve.instancing===!1||!Q.isInstancedMesh&&Ve.instancing===!0||Q.isSkinnedMesh&&Ve.skinning===!1||!Q.isSkinnedMesh&&Ve.skinning===!0||Q.isInstancedMesh&&Ve.instancingColor===!0&&Q.instanceColor===null||Q.isInstancedMesh&&Ve.instancingColor===!1&&Q.instanceColor!==null||Q.isInstancedMesh&&Ve.instancingMorph===!0&&Q.morphTexture===null||Q.isInstancedMesh&&Ve.instancingMorph===!1&&Q.morphTexture!==null||Ve.envMap!==Ye||ne.fog===!0&&Ve.fog!==De||Ve.numClippingPlanes!==void 0&&(Ve.numClippingPlanes!==Se.numPlanes||Ve.numIntersection!==Se.numIntersection)||Ve.vertexAlphas!==tt||Ve.vertexTangents!==st||Ve.morphTargets!==He||Ve.morphNormals!==ft||Ve.morphColors!==Yt||Ve.toneMapping!==Zt||Ve.morphTargetsCount!==mn)&&(it=!0):(it=!0,Ve.__version=ne.version);let Ln=Ve.currentProgram;it===!0&&(Ln=Ea(ne,j,Q));let Yn=!1,Si=!1,Zn=!1;const Ot=Ln.getUniforms(),on=Ve.uniforms;if(We.useProgram(Ln.program)&&(Yn=!0,Si=!0,Zn=!0),ne.id!==ue&&(ue=ne.id,Si=!0),Yn||ee!==R){We.buffers.depth.getReversed()&&R.reversedDepth!==!0&&(R._reversedDepth=!0,R.updateProjectionMatrix()),Ot.setValue(k,"projectionMatrix",R.projectionMatrix),Ot.setValue(k,"viewMatrix",R.matrixWorldInverse);const bi=Ot.map.cameraPosition;bi!==void 0&&bi.setValue(k,xt.setFromMatrixPosition(R.matrixWorld)),Lt.logarithmicDepthBuffer&&Ot.setValue(k,"logDepthBufFC",2/(Math.log(R.far+1)/Math.LN2)),(ne.isMeshPhongMaterial||ne.isMeshToonMaterial||ne.isMeshLambertMaterial||ne.isMeshBasicMaterial||ne.isMeshStandardMaterial||ne.isShaderMaterial)&&Ot.setValue(k,"isOrthographic",R.isOrthographicCamera===!0),ee!==R&&(ee=R,Si=!0,Zn=!0)}if(Ve.needsLights&&(Un.state.directionalShadowMap.length>0&&Ot.setValue(k,"directionalShadowMap",Un.state.directionalShadowMap,q),Un.state.spotShadowMap.length>0&&Ot.setValue(k,"spotShadowMap",Un.state.spotShadowMap,q),Un.state.pointShadowMap.length>0&&Ot.setValue(k,"pointShadowMap",Un.state.pointShadowMap,q)),Q.isSkinnedMesh){Ot.setOptional(k,Q,"bindMatrix"),Ot.setOptional(k,Q,"bindMatrixInverse");const ln=Q.skeleton;ln&&(ln.boneTexture===null&&ln.computeBoneTexture(),Ot.setValue(k,"boneTexture",ln.boneTexture,q))}Q.isBatchedMesh&&(Ot.setOptional(k,Q,"batchingTexture"),Ot.setValue(k,"batchingTexture",Q._matricesTexture,q),Ot.setOptional(k,Q,"batchingIdTexture"),Ot.setValue(k,"batchingIdTexture",Q._indirectTexture,q),Ot.setOptional(k,Q,"batchingColorTexture"),Q._colorsTexture!==null&&Ot.setValue(k,"batchingColorTexture",Q._colorsTexture,q));const On=re.morphAttributes;if((On.position!==void 0||On.normal!==void 0||On.color!==void 0)&&Pe.update(Q,re,Ln),(Si||Ve.receiveShadow!==Q.receiveShadow)&&(Ve.receiveShadow=Q.receiveShadow,Ot.setValue(k,"receiveShadow",Q.receiveShadow)),(ne.isMeshStandardMaterial||ne.isMeshLambertMaterial||ne.isMeshPhongMaterial)&&ne.envMap===null&&j.environment!==null&&(on.envMapIntensity.value=j.environmentIntensity),on.dfgLUT!==void 0&&(on.dfgLUT.value=pA()),Si&&(Ot.setValue(k,"toneMappingExposure",D.toneMappingExposure),Ve.needsLights&&os(on,Zn),De&&ne.fog===!0&&Ze.refreshFogUniforms(on,De),Ze.refreshMaterialUniforms(on,ne,ve,Y,V.state.transmissionRenderTarget[R.id]),qc.upload(k,tl(Ve),on,q)),ne.isShaderMaterial&&ne.uniformsNeedUpdate===!0&&(qc.upload(k,tl(Ve),on,q),ne.uniformsNeedUpdate=!1),ne.isSpriteMaterial&&Ot.setValue(k,"center",Q.center),Ot.setValue(k,"modelViewMatrix",Q.modelViewMatrix),Ot.setValue(k,"normalMatrix",Q.normalMatrix),Ot.setValue(k,"modelMatrix",Q.matrixWorld),ne.isShaderMaterial||ne.isRawShaderMaterial){const ln=ne.uniformsGroups;for(let bi=0,Zi=ln.length;bi<Zi;bi++){const Vs=ln[bi];Ie.update(Vs,Ln),Ie.bind(Vs,Ln)}}return Ln}function os(R,j){R.ambientLightColor.needsUpdate=j,R.lightProbe.needsUpdate=j,R.directionalLights.needsUpdate=j,R.directionalLightShadows.needsUpdate=j,R.pointLights.needsUpdate=j,R.pointLightShadows.needsUpdate=j,R.spotLights.needsUpdate=j,R.spotLightShadows.needsUpdate=j,R.rectAreaLights.needsUpdate=j,R.hemisphereLights.needsUpdate=j}function qr(R){return R.isMeshLambertMaterial||R.isMeshToonMaterial||R.isMeshPhongMaterial||R.isMeshStandardMaterial||R.isShadowMaterial||R.isShaderMaterial&&R.lights===!0}this.getActiveCubeFace=function(){return G},this.getActiveMipmapLevel=function(){return te},this.getRenderTarget=function(){return se},this.setRenderTargetTextures=function(R,j,re){const ne=M.get(R);ne.__autoAllocateDepthBuffer=R.resolveDepthBuffer===!1,ne.__autoAllocateDepthBuffer===!1&&(ne.__useRenderToTexture=!1),M.get(R.texture).__webglTexture=j,M.get(R.depthTexture).__webglTexture=ne.__autoAllocateDepthBuffer?void 0:re,ne.__hasExternalTextures=!0},this.setRenderTargetFramebuffer=function(R,j){const re=M.get(R);re.__webglFramebuffer=j,re.__useDefaultFramebuffer=j===void 0};const Ta=k.createFramebuffer();this.setRenderTarget=function(R,j=0,re=0){se=R,G=j,te=re;let ne=null,Q=!1,De=!1;if(R){const Ne=M.get(R);if(Ne.__useDefaultFramebuffer!==void 0){We.bindFramebuffer(k.FRAMEBUFFER,Ne.__webglFramebuffer),P.copy(R.viewport),z.copy(R.scissor),ce=R.scissorTest,We.viewport(P),We.scissor(z),We.setScissorTest(ce),ue=-1;return}else if(Ne.__webglFramebuffer===void 0)q.setupRenderTarget(R);else if(Ne.__hasExternalTextures)q.rebindTextures(R,M.get(R.texture).__webglTexture,M.get(R.depthTexture).__webglTexture);else if(R.depthBuffer){const tt=R.depthTexture;if(Ne.__boundDepthTexture!==tt){if(tt!==null&&M.has(tt)&&(R.width!==tt.image.width||R.height!==tt.image.height))throw new Error("WebGLRenderTarget: Attached DepthTexture is initialized to the incorrect size.");q.setupDepthRenderbuffer(R)}}const je=R.texture;(je.isData3DTexture||je.isDataArrayTexture||je.isCompressedArrayTexture)&&(De=!0);const Ye=M.get(R).__webglFramebuffer;R.isWebGLCubeRenderTarget?(Array.isArray(Ye[j])?ne=Ye[j][re]:ne=Ye[j],Q=!0):R.samples>0&&q.useMultisampledRTT(R)===!1?ne=M.get(R).__webglMultisampledFramebuffer:Array.isArray(Ye)?ne=Ye[re]:ne=Ye,P.copy(R.viewport),z.copy(R.scissor),ce=R.scissorTest}else P.copy(ie).multiplyScalar(ve).floor(),z.copy(xe).multiplyScalar(ve).floor(),ce=Te;if(re!==0&&(ne=Ta),We.bindFramebuffer(k.FRAMEBUFFER,ne)&&We.drawBuffers(R,ne),We.viewport(P),We.scissor(z),We.setScissorTest(ce),Q){const Ne=M.get(R.texture);k.framebufferTexture2D(k.FRAMEBUFFER,k.COLOR_ATTACHMENT0,k.TEXTURE_CUBE_MAP_POSITIVE_X+j,Ne.__webglTexture,re)}else if(De){const Ne=j;for(let je=0;je<R.textures.length;je++){const Ye=M.get(R.textures[je]);k.framebufferTextureLayer(k.FRAMEBUFFER,k.COLOR_ATTACHMENT0+je,Ye.__webglTexture,re,Ne)}}else if(R!==null&&re!==0){const Ne=M.get(R.texture);k.framebufferTexture2D(k.FRAMEBUFFER,k.COLOR_ATTACHMENT0,k.TEXTURE_2D,Ne.__webglTexture,re)}ue=-1},this.readRenderTargetPixels=function(R,j,re,ne,Q,De,ze,Ne=0){if(!(R&&R.isWebGLRenderTarget)){Dt("WebGLRenderer.readRenderTargetPixels: renderTarget is not THREE.WebGLRenderTarget.");return}let je=M.get(R).__webglFramebuffer;if(R.isWebGLCubeRenderTarget&&ze!==void 0&&(je=je[ze]),je){We.bindFramebuffer(k.FRAMEBUFFER,je);try{const Ye=R.textures[Ne],tt=Ye.format,st=Ye.type;if(R.textures.length>1&&k.readBuffer(k.COLOR_ATTACHMENT0+Ne),!Lt.textureFormatReadable(tt)){Dt("WebGLRenderer.readRenderTargetPixels: renderTarget is not in RGBA or implementation defined format.");return}if(!Lt.textureTypeReadable(st)){Dt("WebGLRenderer.readRenderTargetPixels: renderTarget is not in UnsignedByteType or implementation defined type.");return}j>=0&&j<=R.width-ne&&re>=0&&re<=R.height-Q&&k.readPixels(j,re,ne,Q,we.convert(tt),we.convert(st),De)}finally{const Ye=se!==null?M.get(se).__webglFramebuffer:null;We.bindFramebuffer(k.FRAMEBUFFER,Ye)}}},this.readRenderTargetPixelsAsync=async function(R,j,re,ne,Q,De,ze,Ne=0){if(!(R&&R.isWebGLRenderTarget))throw new Error("THREE.WebGLRenderer.readRenderTargetPixels: renderTarget is not THREE.WebGLRenderTarget.");let je=M.get(R).__webglFramebuffer;if(R.isWebGLCubeRenderTarget&&ze!==void 0&&(je=je[ze]),je)if(j>=0&&j<=R.width-ne&&re>=0&&re<=R.height-Q){We.bindFramebuffer(k.FRAMEBUFFER,je);const Ye=R.textures[Ne],tt=Ye.format,st=Ye.type;if(R.textures.length>1&&k.readBuffer(k.COLOR_ATTACHMENT0+Ne),!Lt.textureFormatReadable(tt))throw new Error("THREE.WebGLRenderer.readRenderTargetPixelsAsync: renderTarget is not in RGBA or implementation defined format.");if(!Lt.textureTypeReadable(st))throw new Error("THREE.WebGLRenderer.readRenderTargetPixelsAsync: renderTarget is not in UnsignedByteType or implementation defined type.");const He=k.createBuffer();k.bindBuffer(k.PIXEL_PACK_BUFFER,He),k.bufferData(k.PIXEL_PACK_BUFFER,De.byteLength,k.STREAM_READ),k.readPixels(j,re,ne,Q,we.convert(tt),we.convert(st),0);const ft=se!==null?M.get(se).__webglFramebuffer:null;We.bindFramebuffer(k.FRAMEBUFFER,ft);const Yt=k.fenceSync(k.SYNC_GPU_COMMANDS_COMPLETE,0);return k.flush(),await US(k,Yt,4),k.bindBuffer(k.PIXEL_PACK_BUFFER,He),k.getBufferSubData(k.PIXEL_PACK_BUFFER,0,De),k.deleteBuffer(He),k.deleteSync(Yt),De}else throw new Error("THREE.WebGLRenderer.readRenderTargetPixelsAsync: requested read bounds are out of range.")},this.copyFramebufferToTexture=function(R,j=null,re=0){const ne=Math.pow(2,-re),Q=Math.floor(R.image.width*ne),De=Math.floor(R.image.height*ne),ze=j!==null?j.x:0,Ne=j!==null?j.y:0;q.setTexture2D(R,0),k.copyTexSubImage2D(k.TEXTURE_2D,re,0,0,ze,Ne,Q,De),We.unbindTexture()};const Aa=k.createFramebuffer(),ls=k.createFramebuffer();this.copyTextureToTexture=function(R,j,re=null,ne=null,Q=0,De=0){let ze,Ne,je,Ye,tt,st,He,ft,Yt;const Zt=R.isCompressedTexture?R.mipmaps[De]:R.image;if(re!==null)ze=re.max.x-re.min.x,Ne=re.max.y-re.min.y,je=re.isBox3?re.max.z-re.min.z:1,Ye=re.min.x,tt=re.min.y,st=re.isBox3?re.min.z:0;else{const on=Math.pow(2,-Q);ze=Math.floor(Zt.width*on),Ne=Math.floor(Zt.height*on),R.isDataArrayTexture?je=Zt.depth:R.isData3DTexture?je=Math.floor(Zt.depth*on):je=1,Ye=0,tt=0,st=0}ne!==null?(He=ne.x,ft=ne.y,Yt=ne.z):(He=0,ft=0,Yt=0);const Rt=we.convert(j.format),mn=we.convert(j.type);let Ve;j.isData3DTexture?(q.setTexture3D(j,0),Ve=k.TEXTURE_3D):j.isDataArrayTexture||j.isCompressedArrayTexture?(q.setTexture2DArray(j,0),Ve=k.TEXTURE_2D_ARRAY):(q.setTexture2D(j,0),Ve=k.TEXTURE_2D),k.pixelStorei(k.UNPACK_FLIP_Y_WEBGL,j.flipY),k.pixelStorei(k.UNPACK_PREMULTIPLY_ALPHA_WEBGL,j.premultiplyAlpha),k.pixelStorei(k.UNPACK_ALIGNMENT,j.unpackAlignment);const Un=k.getParameter(k.UNPACK_ROW_LENGTH),it=k.getParameter(k.UNPACK_IMAGE_HEIGHT),Ln=k.getParameter(k.UNPACK_SKIP_PIXELS),Yn=k.getParameter(k.UNPACK_SKIP_ROWS),Si=k.getParameter(k.UNPACK_SKIP_IMAGES);k.pixelStorei(k.UNPACK_ROW_LENGTH,Zt.width),k.pixelStorei(k.UNPACK_IMAGE_HEIGHT,Zt.height),k.pixelStorei(k.UNPACK_SKIP_PIXELS,Ye),k.pixelStorei(k.UNPACK_SKIP_ROWS,tt),k.pixelStorei(k.UNPACK_SKIP_IMAGES,st);const Zn=R.isDataArrayTexture||R.isData3DTexture,Ot=j.isDataArrayTexture||j.isData3DTexture;if(R.isDepthTexture){const on=M.get(R),On=M.get(j),ln=M.get(on.__renderTarget),bi=M.get(On.__renderTarget);We.bindFramebuffer(k.READ_FRAMEBUFFER,ln.__webglFramebuffer),We.bindFramebuffer(k.DRAW_FRAMEBUFFER,bi.__webglFramebuffer);for(let Zi=0;Zi<je;Zi++)Zn&&(k.framebufferTextureLayer(k.READ_FRAMEBUFFER,k.COLOR_ATTACHMENT0,M.get(R).__webglTexture,Q,st+Zi),k.framebufferTextureLayer(k.DRAW_FRAMEBUFFER,k.COLOR_ATTACHMENT0,M.get(j).__webglTexture,De,Yt+Zi)),k.blitFramebuffer(Ye,tt,ze,Ne,He,ft,ze,Ne,k.DEPTH_BUFFER_BIT,k.NEAREST);We.bindFramebuffer(k.READ_FRAMEBUFFER,null),We.bindFramebuffer(k.DRAW_FRAMEBUFFER,null)}else if(Q!==0||R.isRenderTargetTexture||M.has(R)){const on=M.get(R),On=M.get(j);We.bindFramebuffer(k.READ_FRAMEBUFFER,Aa),We.bindFramebuffer(k.DRAW_FRAMEBUFFER,ls);for(let ln=0;ln<je;ln++)Zn?k.framebufferTextureLayer(k.READ_FRAMEBUFFER,k.COLOR_ATTACHMENT0,on.__webglTexture,Q,st+ln):k.framebufferTexture2D(k.READ_FRAMEBUFFER,k.COLOR_ATTACHMENT0,k.TEXTURE_2D,on.__webglTexture,Q),Ot?k.framebufferTextureLayer(k.DRAW_FRAMEBUFFER,k.COLOR_ATTACHMENT0,On.__webglTexture,De,Yt+ln):k.framebufferTexture2D(k.DRAW_FRAMEBUFFER,k.COLOR_ATTACHMENT0,k.TEXTURE_2D,On.__webglTexture,De),Q!==0?k.blitFramebuffer(Ye,tt,ze,Ne,He,ft,ze,Ne,k.COLOR_BUFFER_BIT,k.NEAREST):Ot?k.copyTexSubImage3D(Ve,De,He,ft,Yt+ln,Ye,tt,ze,Ne):k.copyTexSubImage2D(Ve,De,He,ft,Ye,tt,ze,Ne);We.bindFramebuffer(k.READ_FRAMEBUFFER,null),We.bindFramebuffer(k.DRAW_FRAMEBUFFER,null)}else Ot?R.isDataTexture||R.isData3DTexture?k.texSubImage3D(Ve,De,He,ft,Yt,ze,Ne,je,Rt,mn,Zt.data):j.isCompressedArrayTexture?k.compressedTexSubImage3D(Ve,De,He,ft,Yt,ze,Ne,je,Rt,Zt.data):k.texSubImage3D(Ve,De,He,ft,Yt,ze,Ne,je,Rt,mn,Zt):R.isDataTexture?k.texSubImage2D(k.TEXTURE_2D,De,He,ft,ze,Ne,Rt,mn,Zt.data):R.isCompressedTexture?k.compressedTexSubImage2D(k.TEXTURE_2D,De,He,ft,Zt.width,Zt.height,Rt,Zt.data):k.texSubImage2D(k.TEXTURE_2D,De,He,ft,ze,Ne,Rt,mn,Zt);k.pixelStorei(k.UNPACK_ROW_LENGTH,Un),k.pixelStorei(k.UNPACK_IMAGE_HEIGHT,it),k.pixelStorei(k.UNPACK_SKIP_PIXELS,Ln),k.pixelStorei(k.UNPACK_SKIP_ROWS,Yn),k.pixelStorei(k.UNPACK_SKIP_IMAGES,Si),De===0&&j.generateMipmaps&&k.generateMipmap(Ve),We.unbindTexture()},this.initRenderTarget=function(R){M.get(R).__webglFramebuffer===void 0&&q.setupRenderTarget(R)},this.initTexture=function(R){R.isCubeTexture?q.setTextureCube(R,0):R.isData3DTexture?q.setTexture3D(R,0):R.isDataArrayTexture||R.isCompressedArrayTexture?q.setTexture2DArray(R,0):q.setTexture2D(R,0),We.unbindTexture()},this.resetState=function(){G=0,te=0,se=null,We.reset(),Ae.reset()},typeof __THREE_DEVTOOLS__<"u"&&__THREE_DEVTOOLS__.dispatchEvent(new CustomEvent("observe",{detail:this}))}get coordinateSystem(){return Gi}get outputColorSpace(){return this._outputColorSpace}set outputColorSpace(e){this._outputColorSpace=e;const i=this.getContext();i.drawingBufferColorSpace=Tt._getDrawingBufferColorSpace(e),i.unpackColorSpace=Tt._getUnpackColorSpace()}}const j_={type:"change"},ip={type:"start"},Ov={type:"end"},zc=new ep,W_=new ns,gA=Math.cos(70*PS.DEG2RAD),_n=new K,Wn=2*Math.PI,Vt={NONE:-1,ROTATE:0,DOLLY:1,PAN:2,TOUCH_ROTATE:3,TOUCH_PAN:4,TOUCH_DOLLY_PAN:5,TOUCH_DOLLY_ROTATE:6},Wd=1e-6;class _A extends Sb{constructor(e,i=null){super(e,i),this.state=Vt.NONE,this.target=new K,this.cursor=new K,this.minDistance=0,this.maxDistance=1/0,this.minZoom=0,this.maxZoom=1/0,this.minTargetRadius=0,this.maxTargetRadius=1/0,this.minPolarAngle=0,this.maxPolarAngle=Math.PI,this.minAzimuthAngle=-1/0,this.maxAzimuthAngle=1/0,this.enableDamping=!1,this.dampingFactor=.05,this.enableZoom=!0,this.zoomSpeed=1,this.enableRotate=!0,this.rotateSpeed=1,this.keyRotateSpeed=1,this.enablePan=!0,this.panSpeed=1,this.screenSpacePanning=!0,this.keyPanSpeed=7,this.zoomToCursor=!1,this.autoRotate=!1,this.autoRotateSpeed=2,this.keys={LEFT:"ArrowLeft",UP:"ArrowUp",RIGHT:"ArrowRight",BOTTOM:"ArrowDown"},this.mouseButtons={LEFT:Ir.ROTATE,MIDDLE:Ir.DOLLY,RIGHT:Ir.PAN},this.touches={ONE:Or.ROTATE,TWO:Or.DOLLY_PAN},this.target0=this.target.clone(),this.position0=this.object.position.clone(),this.zoom0=this.object.zoom,this._cursorStyle="auto",this._domElementKeyEvents=null,this._lastPosition=new K,this._lastQuaternion=new rs,this._lastTargetPosition=new K,this._quat=new rs().setFromUnitVectors(e.up,new K(0,1,0)),this._quatInverse=this._quat.clone().invert(),this._spherical=new y_,this._sphericalDelta=new y_,this._scale=1,this._panOffset=new K,this._rotateStart=new ct,this._rotateEnd=new ct,this._rotateDelta=new ct,this._panStart=new ct,this._panEnd=new ct,this._panDelta=new ct,this._dollyStart=new ct,this._dollyEnd=new ct,this._dollyDelta=new ct,this._dollyDirection=new K,this._mouse=new ct,this._performCursorZoom=!1,this._pointers=[],this._pointerPositions={},this._controlActive=!1,this._onPointerMove=xA.bind(this),this._onPointerDown=vA.bind(this),this._onPointerUp=yA.bind(this),this._onContextMenu=RA.bind(this),this._onMouseWheel=MA.bind(this),this._onKeyDown=EA.bind(this),this._onTouchStart=TA.bind(this),this._onTouchMove=AA.bind(this),this._onMouseDown=SA.bind(this),this._onMouseMove=bA.bind(this),this._interceptControlDown=wA.bind(this),this._interceptControlUp=CA.bind(this),this.domElement!==null&&this.connect(this.domElement),this.update()}set cursorStyle(e){this._cursorStyle=e,e==="grab"?this.domElement.style.cursor="grab":this.domElement.style.cursor="auto"}get cursorStyle(){return this._cursorStyle}connect(e){super.connect(e),this.domElement.addEventListener("pointerdown",this._onPointerDown),this.domElement.addEventListener("pointercancel",this._onPointerUp),this.domElement.addEventListener("contextmenu",this._onContextMenu),this.domElement.addEventListener("wheel",this._onMouseWheel,{passive:!1}),this.domElement.getRootNode().addEventListener("keydown",this._interceptControlDown,{passive:!0,capture:!0}),this.domElement.style.touchAction="none"}disconnect(){this.domElement.removeEventListener("pointerdown",this._onPointerDown),this.domElement.ownerDocument.removeEventListener("pointermove",this._onPointerMove),this.domElement.ownerDocument.removeEventListener("pointerup",this._onPointerUp),this.domElement.removeEventListener("pointercancel",this._onPointerUp),this.domElement.removeEventListener("wheel",this._onMouseWheel),this.domElement.removeEventListener("contextmenu",this._onContextMenu),this.stopListenToKeyEvents(),this.domElement.getRootNode().removeEventListener("keydown",this._interceptControlDown,{capture:!0}),this.domElement.style.touchAction="auto"}dispose(){this.disconnect()}getPolarAngle(){return this._spherical.phi}getAzimuthalAngle(){return this._spherical.theta}getDistance(){return this.object.position.distanceTo(this.target)}listenToKeyEvents(e){e.addEventListener("keydown",this._onKeyDown),this._domElementKeyEvents=e}stopListenToKeyEvents(){this._domElementKeyEvents!==null&&(this._domElementKeyEvents.removeEventListener("keydown",this._onKeyDown),this._domElementKeyEvents=null)}saveState(){this.target0.copy(this.target),this.position0.copy(this.object.position),this.zoom0=this.object.zoom}reset(){this.target.copy(this.target0),this.object.position.copy(this.position0),this.object.zoom=this.zoom0,this.object.updateProjectionMatrix(),this.dispatchEvent(j_),this.update(),this.state=Vt.NONE}pan(e,i){this._pan(e,i),this.update()}dollyIn(e){this._dollyIn(e),this.update()}dollyOut(e){this._dollyOut(e),this.update()}rotateLeft(e){this._rotateLeft(e),this.update()}rotateUp(e){this._rotateUp(e),this.update()}update(e=null){const i=this.object.position;_n.copy(i).sub(this.target),_n.applyQuaternion(this._quat),this._spherical.setFromVector3(_n),this.autoRotate&&this.state===Vt.NONE&&this._rotateLeft(this._getAutoRotationAngle(e)),this.enableDamping?(this._spherical.theta+=this._sphericalDelta.theta*this.dampingFactor,this._spherical.phi+=this._sphericalDelta.phi*this.dampingFactor):(this._spherical.theta+=this._sphericalDelta.theta,this._spherical.phi+=this._sphericalDelta.phi);let s=this.minAzimuthAngle,l=this.maxAzimuthAngle;isFinite(s)&&isFinite(l)&&(s<-Math.PI?s+=Wn:s>Math.PI&&(s-=Wn),l<-Math.PI?l+=Wn:l>Math.PI&&(l-=Wn),s<=l?this._spherical.theta=Math.max(s,Math.min(l,this._spherical.theta)):this._spherical.theta=this._spherical.theta>(s+l)/2?Math.max(s,this._spherical.theta):Math.min(l,this._spherical.theta)),this._spherical.phi=Math.max(this.minPolarAngle,Math.min(this.maxPolarAngle,this._spherical.phi)),this._spherical.makeSafe(),this.enableDamping===!0?this.target.addScaledVector(this._panOffset,this.dampingFactor):this.target.add(this._panOffset),this.target.sub(this.cursor),this.target.clampLength(this.minTargetRadius,this.maxTargetRadius),this.target.add(this.cursor);let c=!1;if(this.zoomToCursor&&this._performCursorZoom||this.object.isOrthographicCamera)this._spherical.radius=this._clampDistance(this._spherical.radius);else{const d=this._spherical.radius;this._spherical.radius=this._clampDistance(this._spherical.radius*this._scale),c=d!=this._spherical.radius}if(_n.setFromSpherical(this._spherical),_n.applyQuaternion(this._quatInverse),i.copy(this.target).add(_n),this.object.lookAt(this.target),this.enableDamping===!0?(this._sphericalDelta.theta*=1-this.dampingFactor,this._sphericalDelta.phi*=1-this.dampingFactor,this._panOffset.multiplyScalar(1-this.dampingFactor)):(this._sphericalDelta.set(0,0,0),this._panOffset.set(0,0,0)),this.zoomToCursor&&this._performCursorZoom){let d=null;if(this.object.isPerspectiveCamera){const p=_n.length();d=this._clampDistance(p*this._scale);const m=p-d;this.object.position.addScaledVector(this._dollyDirection,m),this.object.updateMatrixWorld(),c=!!m}else if(this.object.isOrthographicCamera){const p=new K(this._mouse.x,this._mouse.y,0);p.unproject(this.object);const m=this.object.zoom;this.object.zoom=Math.max(this.minZoom,Math.min(this.maxZoom,this.object.zoom/this._scale)),this.object.updateProjectionMatrix(),c=m!==this.object.zoom;const h=new K(this._mouse.x,this._mouse.y,0);h.unproject(this.object),this.object.position.sub(h).add(p),this.object.updateMatrixWorld(),d=_n.length()}else console.warn("WARNING: OrbitControls.js encountered an unknown camera type - zoom to cursor disabled."),this.zoomToCursor=!1;d!==null&&(this.screenSpacePanning?this.target.set(0,0,-1).transformDirection(this.object.matrix).multiplyScalar(d).add(this.object.position):(zc.origin.copy(this.object.position),zc.direction.set(0,0,-1).transformDirection(this.object.matrix),Math.abs(this.object.up.dot(zc.direction))<gA?this.object.lookAt(this.target):(W_.setFromNormalAndCoplanarPoint(this.object.up,this.target),zc.intersectPlane(W_,this.target))))}else if(this.object.isOrthographicCamera){const d=this.object.zoom;this.object.zoom=Math.max(this.minZoom,Math.min(this.maxZoom,this.object.zoom/this._scale)),d!==this.object.zoom&&(this.object.updateProjectionMatrix(),c=!0)}return this._scale=1,this._performCursorZoom=!1,c||this._lastPosition.distanceToSquared(this.object.position)>Wd||8*(1-this._lastQuaternion.dot(this.object.quaternion))>Wd||this._lastTargetPosition.distanceToSquared(this.target)>Wd?(this.dispatchEvent(j_),this._lastPosition.copy(this.object.position),this._lastQuaternion.copy(this.object.quaternion),this._lastTargetPosition.copy(this.target),!0):!1}_getAutoRotationAngle(e){return e!==null?Wn/60*this.autoRotateSpeed*e:Wn/60/60*this.autoRotateSpeed}_getZoomScale(e){const i=Math.abs(e*.01);return Math.pow(.95,this.zoomSpeed*i)}_rotateLeft(e){this._sphericalDelta.theta-=e}_rotateUp(e){this._sphericalDelta.phi-=e}_panLeft(e,i){_n.setFromMatrixColumn(i,0),_n.multiplyScalar(-e),this._panOffset.add(_n)}_panUp(e,i){this.screenSpacePanning===!0?_n.setFromMatrixColumn(i,1):(_n.setFromMatrixColumn(i,0),_n.crossVectors(this.object.up,_n)),_n.multiplyScalar(e),this._panOffset.add(_n)}_pan(e,i){const s=this.domElement;if(this.object.isPerspectiveCamera){const l=this.object.position;_n.copy(l).sub(this.target);let c=_n.length();c*=Math.tan(this.object.fov/2*Math.PI/180),this._panLeft(2*e*c/s.clientHeight,this.object.matrix),this._panUp(2*i*c/s.clientHeight,this.object.matrix)}else this.object.isOrthographicCamera?(this._panLeft(e*(this.object.right-this.object.left)/this.object.zoom/s.clientWidth,this.object.matrix),this._panUp(i*(this.object.top-this.object.bottom)/this.object.zoom/s.clientHeight,this.object.matrix)):(console.warn("WARNING: OrbitControls.js encountered an unknown camera type - pan disabled."),this.enablePan=!1)}_dollyOut(e){this.object.isPerspectiveCamera||this.object.isOrthographicCamera?this._scale/=e:(console.warn("WARNING: OrbitControls.js encountered an unknown camera type - dolly/zoom disabled."),this.enableZoom=!1)}_dollyIn(e){this.object.isPerspectiveCamera||this.object.isOrthographicCamera?this._scale*=e:(console.warn("WARNING: OrbitControls.js encountered an unknown camera type - dolly/zoom disabled."),this.enableZoom=!1)}_updateZoomParameters(e,i){if(!this.zoomToCursor)return;this._performCursorZoom=!0;const s=this.domElement.getBoundingClientRect(),l=e-s.left,c=i-s.top,d=s.width,p=s.height;this._mouse.x=l/d*2-1,this._mouse.y=-(c/p)*2+1,this._dollyDirection.set(this._mouse.x,this._mouse.y,1).unproject(this.object).sub(this.object.position).normalize()}_clampDistance(e){return Math.max(this.minDistance,Math.min(this.maxDistance,e))}_handleMouseDownRotate(e){this._rotateStart.set(e.clientX,e.clientY)}_handleMouseDownDolly(e){this._updateZoomParameters(e.clientX,e.clientX),this._dollyStart.set(e.clientX,e.clientY)}_handleMouseDownPan(e){this._panStart.set(e.clientX,e.clientY)}_handleMouseMoveRotate(e){this._rotateEnd.set(e.clientX,e.clientY),this._rotateDelta.subVectors(this._rotateEnd,this._rotateStart).multiplyScalar(this.rotateSpeed);const i=this.domElement;this._rotateLeft(Wn*this._rotateDelta.x/i.clientHeight),this._rotateUp(Wn*this._rotateDelta.y/i.clientHeight),this._rotateStart.copy(this._rotateEnd),this.update()}_handleMouseMoveDolly(e){this._dollyEnd.set(e.clientX,e.clientY),this._dollyDelta.subVectors(this._dollyEnd,this._dollyStart),this._dollyDelta.y>0?this._dollyOut(this._getZoomScale(this._dollyDelta.y)):this._dollyDelta.y<0&&this._dollyIn(this._getZoomScale(this._dollyDelta.y)),this._dollyStart.copy(this._dollyEnd),this.update()}_handleMouseMovePan(e){this._panEnd.set(e.clientX,e.clientY),this._panDelta.subVectors(this._panEnd,this._panStart).multiplyScalar(this.panSpeed),this._pan(this._panDelta.x,this._panDelta.y),this._panStart.copy(this._panEnd),this.update()}_handleMouseWheel(e){this._updateZoomParameters(e.clientX,e.clientY),e.deltaY<0?this._dollyIn(this._getZoomScale(e.deltaY)):e.deltaY>0&&this._dollyOut(this._getZoomScale(e.deltaY)),this.update()}_handleKeyDown(e){let i=!1;switch(e.code){case this.keys.UP:e.ctrlKey||e.metaKey||e.shiftKey?this.enableRotate&&this._rotateUp(Wn*this.keyRotateSpeed/this.domElement.clientHeight):this.enablePan&&this._pan(0,this.keyPanSpeed),i=!0;break;case this.keys.BOTTOM:e.ctrlKey||e.metaKey||e.shiftKey?this.enableRotate&&this._rotateUp(-Wn*this.keyRotateSpeed/this.domElement.clientHeight):this.enablePan&&this._pan(0,-this.keyPanSpeed),i=!0;break;case this.keys.LEFT:e.ctrlKey||e.metaKey||e.shiftKey?this.enableRotate&&this._rotateLeft(Wn*this.keyRotateSpeed/this.domElement.clientHeight):this.enablePan&&this._pan(this.keyPanSpeed,0),i=!0;break;case this.keys.RIGHT:e.ctrlKey||e.metaKey||e.shiftKey?this.enableRotate&&this._rotateLeft(-Wn*this.keyRotateSpeed/this.domElement.clientHeight):this.enablePan&&this._pan(-this.keyPanSpeed,0),i=!0;break}i&&(e.preventDefault(),this.update())}_handleTouchStartRotate(e){if(this._pointers.length===1)this._rotateStart.set(e.pageX,e.pageY);else{const i=this._getSecondPointerPosition(e),s=.5*(e.pageX+i.x),l=.5*(e.pageY+i.y);this._rotateStart.set(s,l)}}_handleTouchStartPan(e){if(this._pointers.length===1)this._panStart.set(e.pageX,e.pageY);else{const i=this._getSecondPointerPosition(e),s=.5*(e.pageX+i.x),l=.5*(e.pageY+i.y);this._panStart.set(s,l)}}_handleTouchStartDolly(e){const i=this._getSecondPointerPosition(e),s=e.pageX-i.x,l=e.pageY-i.y,c=Math.sqrt(s*s+l*l);this._dollyStart.set(0,c)}_handleTouchStartDollyPan(e){this.enableZoom&&this._handleTouchStartDolly(e),this.enablePan&&this._handleTouchStartPan(e)}_handleTouchStartDollyRotate(e){this.enableZoom&&this._handleTouchStartDolly(e),this.enableRotate&&this._handleTouchStartRotate(e)}_handleTouchMoveRotate(e){if(this._pointers.length==1)this._rotateEnd.set(e.pageX,e.pageY);else{const s=this._getSecondPointerPosition(e),l=.5*(e.pageX+s.x),c=.5*(e.pageY+s.y);this._rotateEnd.set(l,c)}this._rotateDelta.subVectors(this._rotateEnd,this._rotateStart).multiplyScalar(this.rotateSpeed);const i=this.domElement;this._rotateLeft(Wn*this._rotateDelta.x/i.clientHeight),this._rotateUp(Wn*this._rotateDelta.y/i.clientHeight),this._rotateStart.copy(this._rotateEnd)}_handleTouchMovePan(e){if(this._pointers.length===1)this._panEnd.set(e.pageX,e.pageY);else{const i=this._getSecondPointerPosition(e),s=.5*(e.pageX+i.x),l=.5*(e.pageY+i.y);this._panEnd.set(s,l)}this._panDelta.subVectors(this._panEnd,this._panStart).multiplyScalar(this.panSpeed),this._pan(this._panDelta.x,this._panDelta.y),this._panStart.copy(this._panEnd)}_handleTouchMoveDolly(e){const i=this._getSecondPointerPosition(e),s=e.pageX-i.x,l=e.pageY-i.y,c=Math.sqrt(s*s+l*l);this._dollyEnd.set(0,c),this._dollyDelta.set(0,Math.pow(this._dollyEnd.y/this._dollyStart.y,this.zoomSpeed)),this._dollyOut(this._dollyDelta.y),this._dollyStart.copy(this._dollyEnd);const d=(e.pageX+i.x)*.5,p=(e.pageY+i.y)*.5;this._updateZoomParameters(d,p)}_handleTouchMoveDollyPan(e){this.enableZoom&&this._handleTouchMoveDolly(e),this.enablePan&&this._handleTouchMovePan(e)}_handleTouchMoveDollyRotate(e){this.enableZoom&&this._handleTouchMoveDolly(e),this.enableRotate&&this._handleTouchMoveRotate(e)}_addPointer(e){this._pointers.push(e.pointerId)}_removePointer(e){delete this._pointerPositions[e.pointerId];for(let i=0;i<this._pointers.length;i++)if(this._pointers[i]==e.pointerId){this._pointers.splice(i,1);return}}_isTrackingPointer(e){for(let i=0;i<this._pointers.length;i++)if(this._pointers[i]==e.pointerId)return!0;return!1}_trackPointer(e){let i=this._pointerPositions[e.pointerId];i===void 0&&(i=new ct,this._pointerPositions[e.pointerId]=i),i.set(e.pageX,e.pageY)}_getSecondPointerPosition(e){const i=e.pointerId===this._pointers[0]?this._pointers[1]:this._pointers[0];return this._pointerPositions[i]}_customWheelEvent(e){const i=e.deltaMode,s={clientX:e.clientX,clientY:e.clientY,deltaY:e.deltaY};switch(i){case 1:s.deltaY*=16;break;case 2:s.deltaY*=100;break}return e.ctrlKey&&!this._controlActive&&(s.deltaY*=10),s}}function vA(o){this.enabled!==!1&&(this._pointers.length===0&&(this.domElement.setPointerCapture(o.pointerId),this.domElement.ownerDocument.addEventListener("pointermove",this._onPointerMove),this.domElement.ownerDocument.addEventListener("pointerup",this._onPointerUp)),!this._isTrackingPointer(o)&&(this._addPointer(o),o.pointerType==="touch"?this._onTouchStart(o):this._onMouseDown(o),this._cursorStyle==="grab"&&(this.domElement.style.cursor="grabbing")))}function xA(o){this.enabled!==!1&&(o.pointerType==="touch"?this._onTouchMove(o):this._onMouseMove(o))}function yA(o){switch(this._removePointer(o),this._pointers.length){case 0:this.domElement.releasePointerCapture(o.pointerId),this.domElement.ownerDocument.removeEventListener("pointermove",this._onPointerMove),this.domElement.ownerDocument.removeEventListener("pointerup",this._onPointerUp),this.dispatchEvent(Ov),this.state=Vt.NONE,this._cursorStyle==="grab"&&(this.domElement.style.cursor="grab");break;case 1:const e=this._pointers[0],i=this._pointerPositions[e];this._onTouchStart({pointerId:e,pageX:i.x,pageY:i.y});break}}function SA(o){let e;switch(o.button){case 0:e=this.mouseButtons.LEFT;break;case 1:e=this.mouseButtons.MIDDLE;break;case 2:e=this.mouseButtons.RIGHT;break;default:e=-1}switch(e){case Ir.DOLLY:if(this.enableZoom===!1)return;this._handleMouseDownDolly(o),this.state=Vt.DOLLY;break;case Ir.ROTATE:if(o.ctrlKey||o.metaKey||o.shiftKey){if(this.enablePan===!1)return;this._handleMouseDownPan(o),this.state=Vt.PAN}else{if(this.enableRotate===!1)return;this._handleMouseDownRotate(o),this.state=Vt.ROTATE}break;case Ir.PAN:if(o.ctrlKey||o.metaKey||o.shiftKey){if(this.enableRotate===!1)return;this._handleMouseDownRotate(o),this.state=Vt.ROTATE}else{if(this.enablePan===!1)return;this._handleMouseDownPan(o),this.state=Vt.PAN}break;default:this.state=Vt.NONE}this.state!==Vt.NONE&&this.dispatchEvent(ip)}function bA(o){switch(this.state){case Vt.ROTATE:if(this.enableRotate===!1)return;this._handleMouseMoveRotate(o);break;case Vt.DOLLY:if(this.enableZoom===!1)return;this._handleMouseMoveDolly(o);break;case Vt.PAN:if(this.enablePan===!1)return;this._handleMouseMovePan(o);break}}function MA(o){this.enabled===!1||this.enableZoom===!1||this.state!==Vt.NONE||(o.preventDefault(),this.dispatchEvent(ip),this._handleMouseWheel(this._customWheelEvent(o)),this.dispatchEvent(Ov))}function EA(o){this.enabled!==!1&&this._handleKeyDown(o)}function TA(o){switch(this._trackPointer(o),this._pointers.length){case 1:switch(this.touches.ONE){case Or.ROTATE:if(this.enableRotate===!1)return;this._handleTouchStartRotate(o),this.state=Vt.TOUCH_ROTATE;break;case Or.PAN:if(this.enablePan===!1)return;this._handleTouchStartPan(o),this.state=Vt.TOUCH_PAN;break;default:this.state=Vt.NONE}break;case 2:switch(this.touches.TWO){case Or.DOLLY_PAN:if(this.enableZoom===!1&&this.enablePan===!1)return;this._handleTouchStartDollyPan(o),this.state=Vt.TOUCH_DOLLY_PAN;break;case Or.DOLLY_ROTATE:if(this.enableZoom===!1&&this.enableRotate===!1)return;this._handleTouchStartDollyRotate(o),this.state=Vt.TOUCH_DOLLY_ROTATE;break;default:this.state=Vt.NONE}break;default:this.state=Vt.NONE}this.state!==Vt.NONE&&this.dispatchEvent(ip)}function AA(o){switch(this._trackPointer(o),this.state){case Vt.TOUCH_ROTATE:if(this.enableRotate===!1)return;this._handleTouchMoveRotate(o),this.update();break;case Vt.TOUCH_PAN:if(this.enablePan===!1)return;this._handleTouchMovePan(o),this.update();break;case Vt.TOUCH_DOLLY_PAN:if(this.enableZoom===!1&&this.enablePan===!1)return;this._handleTouchMoveDollyPan(o),this.update();break;case Vt.TOUCH_DOLLY_ROTATE:if(this.enableZoom===!1&&this.enableRotate===!1)return;this._handleTouchMoveDollyRotate(o),this.update();break;default:this.state=Vt.NONE}}function RA(o){this.enabled!==!1&&o.preventDefault()}function wA(o){o.key==="Control"&&(this._controlActive=!0,this.domElement.getRootNode().addEventListener("keyup",this._interceptControlUp,{passive:!0,capture:!0}))}function CA(o){o.key==="Control"&&(this._controlActive=!1,this.domElement.getRootNode().removeEventListener("keyup",this._interceptControlUp,{passive:!0,capture:!0}))}function DA({className:o=""}){return O.jsxs("svg",{className:o,viewBox:"0 0 200 200",fill:"none",xmlns:"http://www.w3.org/2000/svg",children:[O.jsx("polygon",{points:"100,10 180,55 180,145 100,190 20,145 20,55",stroke:"url(#grad)",strokeWidth:"4"}),O.jsx("defs",{children:O.jsxs("linearGradient",{id:"grad",x1:"0%",y1:"0%",x2:"100%",y2:"100%",children:[O.jsx("stop",{offset:"0%",stopColor:"#FFD700"}),O.jsx("stop",{offset:"100%",stopColor:"#00E5FF"})]})})]})}function Pv(o){var e,i,s="";if(typeof o=="string"||typeof o=="number")s+=o;else if(typeof o=="object")if(Array.isArray(o)){var l=o.length;for(e=0;e<l;e++)o[e]&&(i=Pv(o[e]))&&(s&&(s+=" "),s+=i)}else for(i in o)o[i]&&(s&&(s+=" "),s+=i);return s}function Ns(){for(var o,e,i=0,s="",l=arguments.length;i<l;i++)(o=arguments[i])&&(e=Pv(o))&&(s&&(s+=" "),s+=e);return s}function Qc({children:o,primary:e,outline:i,className:s="",...l}){return O.jsx("button",{className:Ns("relative overflow-hidden font-display tracking-widest transition transform focus:outline-none focus:ring",e&&"bg-gradient-to-tr from-gold to-cyan text-black shadow-lg",i&&"border border-cream text-cream hover:bg-cream/10","px-6 py-3 rounded-lg","active:scale-95",s),...l,children:o})}function NA(){return O.jsxs("div",{className:"flex flex-col items-center gap-2 animate-fade-up cursor-pointer",children:[O.jsx("div",{className:"w-px h-12 bg-gradient-to-b from-gold/50 to-transparent animate-pulse"}),O.jsx("span",{className:"text-xs uppercase opacity-70",children:"Scroll"})]})}function Gh({text:o}){return O.jsxs("div",{className:"flex items-center gap-3 mb-4",children:[O.jsx("span",{className:"text-xs uppercase tracking-widest text-gold",children:o}),O.jsx("div",{className:"flex-1 h-px bg-gold/50"})]})}function Bc({icon:o,title:e,text:i}){return O.jsxs("div",{className:"flex items-start gap-4",children:[O.jsx("span",{className:"text-2xl leading-none",children:o}),O.jsxs("div",{children:[O.jsx("p",{className:"font-display font-bold text-base mb-1",children:e}),O.jsx("p",{className:"text-sm opacity-70 leading-relaxed",children:i})]})]})}function UA(){const o=Wt.useRef(),e=Wt.useRef();return Wt.useEffect(()=>{const i=()=>{const s=window.scrollY/window.innerHeight;o.current.style.setProperty("--ripple",Math.min(1,s))};return window.addEventListener("scroll",i),()=>window.removeEventListener("scroll",i)},[]),Wt.useEffect(()=>{const i=e.current,s=new ZS;s.fog=new $h(8,.015);const l=new si(35,i.clientWidth/i.clientHeight,.1,1e3);l.position.set(0,0,8);const c=new mA({canvas:i,antialias:!0});c.setSize(i.clientWidth,i.clientHeight),c.setPixelRatio(window.devicePixelRatio);const d=new vi,p=15e3,m=new Float32Array(p*3);for(let U=0;U<p;U++)m[U*3+0]=(Math.random()-.5)*40,m[U*3+1]=(Math.random()-.5)*40,m[U*3+2]=(Math.random()-.5)*40;d.setAttribute("position",new Ni(m,3));const h=new Sv({size:.05,color:58879,transparent:!0,opacity:.6,blending:qd}),v=new sb(d,h);s.add(v);const y=new np(1.2,.3,200,32),g=new db({color:16766720,metalness:.7,roughness:.1,emissive:16749824,emissiveIntensity:.6}),x=new Wi(y,g);s.add(x);const E=new vb(2236962);s.add(E);const w=new _b(58879,2,50);w.position.set(5,5,5),s.add(w);const b=new _A(l,i);b.enableZoom=!1,b.enablePan=!1,b.autoRotate=!0,b.autoRotateSpeed=.2;const S=()=>{const{clientWidth:U,clientHeight:N}=i;l.aspect=U/N,l.updateProjectionMatrix(),c.setSize(U,N)};window.addEventListener("resize",S);const C=()=>{x.rotation.x+=.002,x.rotation.y+=.003,v.rotation.y+=5e-4,b.update(),c.render(s,l),requestAnimationFrame(C)};return C(),()=>{window.removeEventListener("resize",S),c.dispose()}},[]),O.jsxs("main",{className:"bg-ink text-cream font-body overflow-x-hidden relative",children:[O.jsx("canvas",{ref:e,className:"absolute inset-0 z-0"}),O.jsxs("section",{ref:o,className:"relative min-h-screen flex flex-col justify-center items-center text-center px-6 z-10",style:{"--ripple":0},children:[O.jsx("div",{className:"absolute inset-0 pointer-events-none",children:O.jsx(DA,{className:"w-96 h-96 animate-spin-slow"})}),O.jsxs("p",{className:"badge mb-8",children:[O.jsx("span",{className:"pulse"})," Live mainnet – 42 validators"]}),O.jsx("h1",{className:"display text-6xl lg:text-9xl leading-tight gradient-text drop-shadow-xl",style:{transform:"scale(calc(1 + var(--ripple) * .1))",filter:"blur(calc(var(--ripple) * 1px))"},children:"X3STAR"}),O.jsxs("p",{className:"max-w-xl mt-6 opacity-60 leading-relaxed",children:["An ocean‑hardened, atomic‑swap middleware blockchain."," ",O.jsx("strong",{className:"text-cyan",children:"Valencia testnet live."})]}),O.jsxs("div",{className:"mt-12 flex flex-wrap justify-center gap-4",children:[O.jsx(Qc,{primary:!0,children:"Get started"}),O.jsx(Qc,{outline:!0,children:"Read the whitepaper"})]}),O.jsx(NA,{})]}),O.jsxs("section",{id:"about",className:"section",children:[O.jsx(Gh,{text:"Problem"}),O.jsx("h2",{className:"section-title",children:"Interoperability is a leak."}),O.jsxs("p",{className:"section-sub",children:["Projects force users to jump between incompatible VMs, losing atomicity and custody. X3 plugs between chains and ",O.jsx("strong",{className:"text-cyan",children:"guarantees"})," cross‑VM trades in a single transaction."]}),O.jsxs("div",{className:"grid lg:grid-cols-2 gap-16 mt-12",children:[O.jsx(Bc,{icon:"🔥",title:"Slippage",text:"Users bleed value across hops."}),O.jsx(Bc,{icon:"⛓️",title:"Complexity",text:"Smart contracts become stitching messes."}),O.jsx(Bc,{icon:"👁️",title:"Opacity",text:"You never know which chain your assets took."}),O.jsx(Bc,{icon:"🐢",title:"Latency",text:"Blocks per hop stack to minutes."})]})]}),O.jsxs("section",{id:"tech",className:"tech-section",children:[O.jsx(Gh,{text:"Tech specs"}),O.jsx("div",{className:"grid lg:grid-cols-3 gap-2 bg-surface-dark rounded-3xl overflow-hidden mt-12",children:[{n:"1M",label:"TPS"},{n:"12s",label:"Finality"},{n:"0.0001ꜩ",label:"Tx cost"}].map((i,s)=>O.jsxs("div",{className:"tech-card group",children:[O.jsx("span",{className:"tc-number",children:i.n}),O.jsx("span",{className:"tc-label",children:i.label}),O.jsx("div",{className:"tc-bar mt-4",children:O.jsx("div",{className:"tc-bar-fill group-hover:w-full transition-[width] duration-1500 ease-[var(--easing)]"})})]},s))})]})]})}const Pr=[{id:"core",label:"Core Surfaces",summary:"Safe shells that can absorb stable contracts later without needing live chain access now.",routes:[{id:"wallet-home",label:"Wallet Home",stage:"shell-only",description:"Portfolio frame, signing boundary copy, balance narratives, and alert choreography.",readiness:"Design-safe now",blockedBy:["Live wallet signing contracts","Custody ownership freeze"],metrics:[{label:"Draft modules",value:"4"},{label:"Contract coupling",value:"None"},{label:"Fixture scope",value:"User-only"}],screens:[{id:"overview",label:"Overview",kicker:"Account frame",headline:"Show balances and trust boundaries before asking users to sign anything.",summary:"This view tests how much chain posture, warnings, and account context fit above the fold before the first transactional prompt.",cards:[{label:"Portfolio spread",value:"$184.2k",tone:"warm",detail:"Mock valuation over four custody classes."},{label:"Available liquidity",value:"$37.6k",tone:"neutral",detail:"Immediate user-controlled balance only."},{label:"Signing routes",value:"2 / 5",tone:"alert",detail:"Only user-signed paths should appear here."}],modules:[{name:"Balance shelf",status:"ready for shell",detail:"Token grouping, balance ladder, and fiat framing."},{name:"Trust banner",status:"ready for shell",detail:"Explains what the wallet will never sign on behalf of backend operators."},{name:"Action rail",status:"blocked",detail:"Real send, swap, and bridge actions stay disabled until signer ownership freezes."}]},{id:"activity",label:"Activity",kicker:"History shell",headline:"Design the cadence of an activity ledger before event truth is stable.",summary:"This tests density, status language, and empty-state choreography using local fixture events only.",cards:[{label:"Recent items",value:"18",tone:"neutral",detail:"Mock events across send, stake, and review states."},{label:"Status families",value:"6",tone:"cool",detail:"Pending, signed, observed, delayed, escalated, closed."},{label:"Needs model freeze",value:"Yes",tone:"alert",detail:"Final event taxonomies still depend on indexer freeze."}],modules:[{name:"Timeline cards",status:"ready for shell",detail:"Visual rhythm and copy can be tuned now."},{name:"Filters",status:"ready for shell",detail:"By asset, by route, by risk."},{name:"Deep links",status:"blocked",detail:"Cannot bind to explorer detail pages yet."}]},{id:"risk",label:"Risk & trust",kicker:"Boundary education",headline:"Put signing, pause, and custody boundaries in plain language.",summary:"This screen exists to pressure-test the copy that separates user keys from backend-controlled protocol keys.",cards:[{label:"User-signed actions",value:"Send / approve",tone:"cool",detail:"Only user-controlled keys belong here."},{label:"Backend-owned actions",value:"Relayer / treasury",tone:"warm",detail:"Must route through custody-service."},{label:"Misleading copy risk",value:"High",tone:"alert",detail:"Reason durable wallet UX still waits."}],modules:[{name:"Boundary glossary",status:"ready for shell",detail:"Lets product and security align language early."},{name:"Incident states",status:"shell-only",detail:"Mock pause and custody outage messaging."},{name:"Recovery flows",status:"blocked",detail:"Need confirmed backend fallbacks and ownership."}]}],scenarios:[{id:"calm",label:"Calm market",state:"safe now",description:"Default healthy account posture with no action pressure."},{id:"stressed",label:"Signer uncertainty",state:"copy test",description:"Stress-tests warnings around provisional signing semantics."},{id:"paused",label:"Protocol pause",state:"education",description:"Explains what the wallet can still show when protocol actions are blocked."}],journey:[{title:"Enter account",state:"ready",detail:"Identity and balances are visible without backend mutation."},{title:"Evaluate trust banner",state:"ready",detail:"Boundary copy can be refined immediately."},{title:"Attempt action",state:"blocked",detail:"Real mutations remain disabled until signing freezes."}]},{id:"network-overview",label:"Network Overview",stage:"shell-only",description:"Public-facing chain posture, release language, and module scorecard composition.",readiness:"Design-safe now",blockedBy:["Frozen RPC and sidecar contract set"],metrics:[{label:"Draft modules",value:"4"},{label:"Contract coupling",value:"Low"},{label:"Fixture scope",value:"Network-wide"}],screens:[{id:"command",label:"Command deck",kicker:"Public posture",headline:"Let visitors understand the chain narrative before exposing raw protocol state.",summary:"This tests the hierarchy between readiness, proof systems, validator signals, and chain identity.",cards:[{label:"Public readiness rail",value:"5 modules",tone:"cool",detail:"Consensus, bridge, wallet, ops, launch."},{label:"Narrative confidence",value:"72%",tone:"warm",detail:"How much copy can stay honest without going technical."},{label:"Contract dependency",value:"RPC pack",tone:"neutral",detail:"Needs frozen read models, not write flows."}],modules:[{name:"Hero and readiness rail",status:"ready for shell",detail:"Works entirely on fixture language."},{name:"Module scorecards",status:"shell-only",detail:"Can later bind to LaunchOps or sidecar summaries."},{name:"Live telemetry",status:"blocked",detail:"Needs explicit consumer contract and ownership."}]},{id:"modules",label:"Module scorecards",kicker:"Subsystem map",headline:"Test how much operational truth a public dashboard can carry without collapsing into noise.",summary:"This screen pressures card density, status legend clarity, and how technical to make module names for different audiences.",cards:[{label:"Subsystem cards",value:"8",tone:"neutral",detail:"Mocked cards for bridge, wallet, verifier, and GPU paths."},{label:"Status legend",value:"4 states",tone:"cool",detail:"Ready, partial, blocked, downstream."},{label:"Live data need",value:"Deferred",tone:"alert",detail:"Stable query pack still pending."}],modules:[{name:"Status legend",status:"ready for shell",detail:"No chain bindings required."},{name:"Readiness meter",status:"ready for shell",detail:"Can later bind to LaunchOps or sidecar summary."},{name:"Validator pulse",status:"blocked",detail:"Needs explicit consumer contract and ownership."}]},{id:"cta",label:"Action band",kicker:"Public next step",headline:"Design the call-to-action layer without committing to a backend promise surface.",summary:"This view isolates launch-oriented CTAs, role-specific onboarding, and risk language for testnet versus mainnet visitors.",cards:[{label:"CTA lanes",value:"3",tone:"warm",detail:"Builders, validators, operators."},{label:"Copy variants",value:"9",tone:"neutral",detail:"Role-specific language paths."},{label:"Backend risk",value:"Low",tone:"cool",detail:"Mostly information architecture."}],modules:[{name:"Role selector",status:"ready for shell",detail:"Pure IA problem."},{name:"Eligibility copy",status:"shell-only",detail:"Can be tuned before live program state exists."},{name:"Live forms",status:"blocked",detail:"Need confirmed APIs and operational flow."}]}],scenarios:[{id:"public",label:"Public visitor",state:"narrative",description:"Focuses on clarity and credibility over protocol detail."},{id:"operator",label:"Operator lens",state:"technical",description:"Pushes denser module labeling and more specific readiness copy."},{id:"launch",label:"Launch week",state:"event mode",description:"Stresses announcement rails and urgency placement."}],journey:[{title:"Read chain posture",state:"ready",detail:"Narrative and hierarchy can be tuned now."},{title:"Inspect subsystems",state:"ready",detail:"Module cards can stay fixture-backed."},{title:"Query live state",state:"blocked",detail:"Needs finalized read contracts."}]}]},{id:"deferred",label:"Deferred Until Freeze",summary:"High-risk flows remain visible for planning, but stay mock-only until the backend contracts stop moving.",routes:[{id:"bridge-status",label:"Bridge Status",stage:"blocked",description:"Deposit, refund, timeout, and settlement ladder for cross-chain sessions.",readiness:"Blocked",blockedBy:["Bridge and relayer lifecycle freeze","Indexer event model freeze"],metrics:[{label:"Draft modules",value:"5"},{label:"Contract coupling",value:"High"},{label:"Fixture scope",value:"Bridge journey"}],screens:[{id:"session-board",label:"Session board",kicker:"Bridge queue",headline:"Test how much lifecycle detail fits before the board becomes an operator console.",summary:"This shell compares public user status against the denser operator context that the real sidecar will eventually need to provide.",cards:[{label:"Mock sessions",value:"14",tone:"neutral",detail:"Across pending, proving, timed out, and refunded."},{label:"Lifecycle rungs",value:"7",tone:"warm",detail:"From source lock to settlement or refund."},{label:"Truth source",value:"Unfrozen",tone:"alert",detail:"Still blocked on relayer and event freeze."}],modules:[{name:"Session list",status:"shell-only",detail:"IA and status vocabulary can be refined now."},{name:"Source/target chips",status:"ready for shell",detail:"Good place to settle chain icon and naming patterns."},{name:"Real progress state",status:"blocked",detail:"Needs authoritative relayer and settlement lifecycle."}]},{id:"lifecycle",label:"Lifecycle ladder",kicker:"State machine",headline:"Model every bridge step visually before the backend chooses the final event families.",summary:"This tests the legibility of timeouts, duplicate rejection, pause, and refund outcomes without claiming any final state names yet.",cards:[{label:"Visible transitions",value:"9",tone:"cool",detail:"Designed to compress into a mobile timeline."},{label:"Refund branches",value:"2",tone:"warm",detail:"Timeout and rejection flows remain mock-only."},{label:"Semantic stability",value:"Low",tone:"alert",detail:"Final lifecycle names still belong to backend."}],modules:[{name:"Step ladder",status:"shell-only",detail:"Pure IA and motion testing."},{name:"Failure states",status:"shell-only",detail:"Copy stress-test for refunds and disputes."},{name:"Progress polling",status:"blocked",detail:"Needs sidecar and indexer contracts."}]},{id:"incident",label:"Pause and incident",kicker:"Exceptional state",headline:"Make pause, degraded relayer health, and operator intervention legible before implementation.",summary:"This shell exists to test warning choreography and who sees which level of detail when bridge operations are degraded.",cards:[{label:"Incident tiers",value:"3",tone:"alert",detail:"Informational, degraded, paused."},{label:"Banner variants",value:"6",tone:"neutral",detail:"Role-based language variants."},{label:"Backend dependency",value:"Critical",tone:"alert",detail:"Requires authoritative pause semantics."}],modules:[{name:"Incident banner",status:"ready for shell",detail:"UI can be refined immediately."},{name:"User guidance",status:"shell-only",detail:"Who should wait, retry, or contact support."},{name:"Operator controls",status:"blocked",detail:"Not a frontend problem until governance pause is frozen."}]}],scenarios:[{id:"steady",label:"Steady flow",state:"baseline",description:"Healthy relayer and normal settlement timing."},{id:"timeout",label:"Timeout path",state:"stress",description:"Tests how refunds and elapsed timers should read."},{id:"paused",label:"Pause active",state:"critical",description:"Tests what the UI should hide versus keep visible."}],journey:[{title:"Lock source asset",state:"blocked",detail:"Needs final bridge contract semantics."},{title:"Observe proof progression",state:"blocked",detail:"Needs relayer lifecycle and event freeze."},{title:"Complete or refund",state:"blocked",detail:"Cannot be made durable before timeout and refund logic settle."}]},{id:"explorer",label:"Explorer Feed",stage:"blocked",description:"Block, transaction, account, and event timeline shell for public chain observability.",readiness:"Blocked",blockedBy:["Indexer event model freeze"],metrics:[{label:"Draft modules",value:"4"},{label:"Contract coupling",value:"High"},{label:"Fixture scope",value:"Event taxonomy"}],screens:[{id:"blocks",label:"Blocks",kicker:"Block stream",headline:"Set the visual rhythm for blocks and validator attribution without assuming final event fields.",summary:"This shell tests density and scannability for block cards, validator hints, and finality badges.",cards:[{label:"Block cards",value:"20",tone:"cool",detail:"Mock stream with finality and validator hints."},{label:"Badge variants",value:"5",tone:"neutral",detail:"Final, pending, delayed, challenged, archived."},{label:"Producer truth",value:"Unfrozen",tone:"alert",detail:"Still dependent on canonical event producers."}],modules:[{name:"Block card system",status:"ready for shell",detail:"Spacing and hierarchy can be tuned now."},{name:"Validator chips",status:"shell-only",detail:"Useful for density testing only."},{name:"Event joins",status:"blocked",detail:"Need stable correlation ids."}]},{id:"activity",label:"Activity feed",kicker:"Cross-domain events",headline:"Pressure-test event language before one lifecycle becomes many incompatible UI labels.",summary:"This shell helps settle vocabulary and visual grouping while the backend chooses the final canonical event families.",cards:[{label:"Event families",value:"6",tone:"neutral",detail:"Wallet, verifier, settlement, governance, relayer, ops."},{label:"Grouping patterns",value:"3",tone:"warm",detail:"Time, domain, entity."},{label:"Freeze dependency",value:"Hard",tone:"alert",detail:"Needs event family and field stability."}],modules:[{name:"Family grouping",status:"shell-only",detail:"IA useful before any schema exists."},{name:"Correlation chips",status:"blocked",detail:"Need stable event identifiers."},{name:"Deep links",status:"blocked",detail:"Need explorer entity model and routes."}]},{id:"entity",label:"Entity detail",kicker:"Address and transaction detail",headline:"Design detail pages without lying about fields we do not own yet.",summary:"This screen is for layout and navigation architecture only until the event and entity model are frozen.",cards:[{label:"Detail modules",value:"5",tone:"cool",detail:"Header, status, trace, related items, metadata."},{label:"Identity keys",value:"TBD",tone:"alert",detail:"Cannot claim final joins yet."},{label:"Route readiness",value:"IA only",tone:"neutral",detail:"Pure shell until event model freeze."}],modules:[{name:"Header anatomy",status:"ready for shell",detail:"Can refine without live schemas."},{name:"Trace ladder",status:"shell-only",detail:"Visual shape only."},{name:"Canonical IDs",status:"blocked",detail:"Backend still owns identity semantics."}]}],scenarios:[{id:"dense",label:"Dense event day",state:"load test",description:"Tests visual rhythm under heavy event volume."},{id:"quiet",label:"Quiet chain period",state:"empty state",description:"Tests whether sparse telemetry still feels intentional."},{id:"incident",label:"Incident clustering",state:"stress",description:"Tests how exceptional event families should stack."}],journey:[{title:"Read block stream",state:"blocked",detail:"Needs final block and event fields."},{title:"Drill into entity",state:"blocked",detail:"Needs stable identity keys and entity model."},{title:"Correlate cross-domain events",state:"blocked",detail:"Needs canonical event families and joins."}]},{id:"governance",label:"Governance Desk",stage:"blocked",description:"Proposal board, vote detail, treasury lane, and validator oversight shells.",readiness:"Blocked",blockedBy:["Wallet and custody boundary freeze","Indexer event model freeze"],metrics:[{label:"Draft modules",value:"4"},{label:"Contract coupling",value:"Medium"},{label:"Fixture scope",value:"Governance models"}],screens:[{id:"proposal-board",label:"Proposal board",kicker:"Decision surface",headline:"Test how proposals, disputes, and treasury changes compete for attention.",summary:"This shell helps sort information hierarchy for governance without binding to any live vote, signer, or treasury contract yet.",cards:[{label:"Board lanes",value:"3",tone:"neutral",detail:"Active, review, archived."},{label:"Priority rules",value:"4",tone:"warm",detail:"Dispute, treasury, parameter, emergency."},{label:"Trust dependency",value:"Medium",tone:"alert",detail:"Needs signer and event boundaries to settle."}],modules:[{name:"Proposal cards",status:"ready for shell",detail:"Board design can be tested now."},{name:"Escalation labels",status:"shell-only",detail:"Copy and severity mapping only."},{name:"Real vote clocks",status:"blocked",detail:"Need authoritative governance timing semantics."}]},{id:"vote-detail",label:"Vote detail",kicker:"Decision anatomy",headline:"Design a decision page that distinguishes user intent from backend authority.",summary:"This shell separates what the user can sign from what custody or governance-controlled services own.",cards:[{label:"Decision modules",value:"5",tone:"cool",detail:"Summary, rationale, quorum, timeline, actor map."},{label:"Signer boundaries",value:"Unsettled",tone:"alert",detail:"Wallet and custody freeze still needed."},{label:"Copy variants",value:"7",tone:"neutral",detail:"Different language for delegates, operators, and observers."}],modules:[{name:"Vote anatomy",status:"shell-only",detail:"Page structure can be refined now."},{name:"Participation rail",status:"blocked",detail:"Needs final signing and role ownership."},{name:"Treasury implications",status:"blocked",detail:"Needs custody-backed treasury semantics."}]},{id:"oversight",label:"Validator oversight",kicker:"Operational governance",headline:"Explore how governance and validator oversight share a surface without confusing the operator role.",summary:"This shell lets product and protocol teams test whether validator, dispute, and treasury oversight belong together or should split.",cards:[{label:"Oversight cards",value:"6",tone:"neutral",detail:"Health, disputes, quorum, treasury, emergency, audits."},{label:"Actor roles",value:"4",tone:"cool",detail:"User, delegate, operator, governance signer."},{label:"Backend certainty",value:"Partial",tone:"alert",detail:"Still depends on event and signer freeze."}],modules:[{name:"Oversight matrix",status:"ready for shell",detail:"IA and role grouping can be tested now."},{name:"Operational drilldown",status:"shell-only",detail:"Only visual hierarchy today."},{name:"Action affordances",status:"blocked",detail:"Do not ship until signer and event truth settle."}]}],scenarios:[{id:"routine",label:"Routine governance",state:"baseline",description:"Healthy proposal flow with low urgency."},{id:"dispute",label:"Dispute escalation",state:"stress",description:"Tests urgency and decision framing."},{id:"treasury",label:"Treasury review",state:"financial",description:"Tests visibility for spend and signer boundaries."}],journey:[{title:"Scan board",state:"ready",detail:"Layout and severity hierarchy can be refined today."},{title:"Inspect vote detail",state:"blocked",detail:"Needs durable signing ownership."},{title:"Act on governance item",state:"blocked",detail:"Not safe until wallet/custody boundary freezes."}]}]}],LA=[{gate:"Runtime API freeze",status:"near",note:"Inventory work exists, but live-code validation currently shows doc and implementation drift."},{gate:"RPC and sidecar contracts",status:"closest",note:"The strongest frontend-facing material exists here, but it still depends on runtime reconciliation."},{gate:"Bridge and relayer lifecycle",status:"blocked",note:"Replay, timeout, pause, and settlement semantics still belong to backend cleanup."},{gate:"Wallet and custody boundary",status:"partial",note:"Boundary is documented, but signer-path enforcement still needs proof in live code."},{gate:"Indexer event model",status:"downstream",note:"Cannot freeze honestly before runtime, bridge, and signer semantics stop moving."}],OA=["This shell never fetches from RPC, sidecar, gateway, or indexer endpoints.","Every metric and status in this surface is fixture data for IA and local state transition testing only.","Blocked areas remain visible so navigation, severity language, and route density can be tested without promising backend readiness."],PA=[{title:"Account Entry",state:"safe now",description:"Identity, balances, notices, and trust framing can be refined without chain coupling."},{title:"Bridge Journey",state:"wait",description:"Session state, refund logic, and pause behavior stay mocked until the relayer lifecycle freezes."},{title:"Explorer Narrative",state:"wait",description:"Event cards and entity drilldowns stay schematic until the event model is versioned."}],IA=[],FA={routes:IA};function zA(o){const e=Array.isArray(o.allowed_methods)?[...o.allowed_methods]:[];return{routeId:o.route_id,routeLabel:o.route_label,rationale:o.rationale,allowedMethods:e,directReadCount:e.length,enforcementMode:e.length>0?"direct-read-guarded":"sidecar-only"}}const BA=(FA.routes??[]).map(zA),HA=new Map(BA.map(o=>[o.routeId,o]));function q_(o){return HA.get(o)??{routeId:o,routeLabel:o,rationale:"No generated route contract entry is available for this shell route yet.",allowedMethods:[],directReadCount:0,enforcementMode:"sidecar-only"}}function GA(o){for(const i of Pr){const s=i.routes.find(l=>l.id===o);if(s)return{...s,groupLabel:i.label,groupSummary:i.summary}}const e=Pr[0];return{...e.routes[0],groupLabel:e.label,groupSummary:e.summary}}function VA(){const[o,e]=Wt.useState(Pr[0].routes[0].id),[i,s]=Wt.useState(Pr[0].routes[0].screens[0].id),[l,c]=Wt.useState(Pr[0].routes[0].scenarios[0].id),[d,p]=Wt.useState(0),m=Wt.useMemo(()=>GA(o),[o]),h=Wt.useMemo(()=>q_(o),[o]);Wt.useEffect(()=>{s(m.screens[0].id),c(m.scenarios[0].id),p(0)},[m]);const v=Wt.useMemo(()=>m.screens.find(x=>x.id===i)??m.screens[0],[m,i]),y=Wt.useMemo(()=>m.scenarios.find(x=>x.id===l)??m.scenarios[0],[m,l]),g=m.journey[d]??m.journey[0];return O.jsx("main",{className:"min-h-screen bg-[#0b0911] text-[#f4efe5]",children:O.jsxs("div",{className:"mx-auto flex min-h-screen max-w-[1600px] flex-col gap-8 px-5 py-6 lg:px-8",children:[O.jsx("header",{className:"overflow-hidden rounded-[32px] border border-white/10 bg-[radial-gradient(circle_at_top_left,_rgba(244,151,72,0.22),_transparent_38%),linear-gradient(135deg,_rgba(23,17,27,0.98),_rgba(11,9,17,1))] p-6 shadow-[0_24px_80px_rgba(0,0,0,0.45)] lg:p-8",children:O.jsxs("div",{className:"flex flex-col gap-6 lg:flex-row lg:items-end lg:justify-between",children:[O.jsxs("div",{className:"max-w-3xl space-y-4",children:[O.jsx("div",{className:"inline-flex items-center gap-2 rounded-full border border-[#f49748]/35 bg-[#f49748]/10 px-3 py-1 text-[11px] uppercase tracking-[0.28em] text-[#ffd2ab]",children:"Mock Contract Mode"}),O.jsxs("div",{className:"space-y-3",children:[O.jsx("p",{className:"text-sm uppercase tracking-[0.35em] text-[#9f9487]",children:"Disposable frontend shell"}),O.jsx("h1",{className:"max-w-2xl text-4xl font-semibold leading-tight text-[#fbf6ef] lg:text-6xl",children:"X3 can rehearse route architecture now without binding itself to unstable protocol contracts."}),O.jsx("p",{className:"max-w-2xl text-base leading-7 text-[#c5b7a7] lg:text-lg",children:"This surface stays local, fixture-backed, and intentionally disposable. It exists to refine information architecture, copy boundaries, route density, and interaction pacing while backend freeze work closes the real contract set."})]})]}),O.jsx("div",{className:"grid gap-3 rounded-[28px] border border-white/8 bg-black/20 p-4 backdrop-blur md:grid-cols-2 lg:min-w-[420px]",children:LA.map(x=>O.jsxs("div",{className:"rounded-[22px] border border-white/8 bg-white/[0.03] p-4",children:[O.jsx("p",{className:"text-[11px] uppercase tracking-[0.28em] text-[#9f9487]",children:x.status}),O.jsx("h2",{className:"mt-2 text-base font-medium text-[#fbf6ef]",children:x.gate}),O.jsx("p",{className:"mt-2 text-sm leading-6 text-[#b7aa9d]",children:x.note})]},x.gate))})]})}),O.jsxs("section",{className:"grid gap-6 lg:grid-cols-[320px_minmax(0,1fr)] xl:grid-cols-[360px_minmax(0,1fr)]",children:[O.jsxs("aside",{className:"rounded-[28px] border border-white/8 bg-[#14111a] p-4 shadow-[0_18px_50px_rgba(0,0,0,0.32)] lg:p-5",children:[O.jsxs("div",{className:"mb-5 flex items-center justify-between gap-3",children:[O.jsxs("div",{children:[O.jsx("p",{className:"text-[11px] uppercase tracking-[0.28em] text-[#9f9487]",children:"Route map"}),O.jsx("h2",{className:"mt-2 text-xl font-medium text-[#fbf6ef]",children:"Frontend shells"})]}),O.jsx("div",{className:"rounded-full border border-[#5bc0be]/25 bg-[#5bc0be]/10 px-3 py-1 text-xs uppercase tracking-[0.22em] text-[#a8f2ec]",children:"fixture only"})]}),O.jsx("div",{className:"space-y-5",children:Pr.map(x=>O.jsxs("section",{className:"space-y-3",children:[O.jsxs("div",{children:[O.jsx("h3",{className:"text-sm font-semibold uppercase tracking-[0.22em] text-[#f1b780]",children:x.label}),O.jsx("p",{className:"mt-1 text-sm leading-6 text-[#9f9487]",children:x.summary})]}),O.jsx("div",{className:"space-y-2",children:x.routes.map(E=>{const w=E.id===o,b=q_(E.id);return O.jsxs("button",{type:"button",onClick:()=>e(E.id),className:Ns("w-full rounded-[22px] border px-4 py-4 text-left transition duration-300 focus:outline-none focus-visible:ring-2 focus-visible:ring-[#f49748]",w?"border-[#f49748]/45 bg-[#f49748]/10 shadow-[0_12px_30px_rgba(244,151,72,0.12)]":"border-white/7 bg-white/[0.02] hover:border-white/18 hover:bg-white/[0.04]"),children:[O.jsxs("div",{className:"flex items-start justify-between gap-3",children:[O.jsxs("div",{children:[O.jsx("p",{className:"text-sm font-medium text-[#fbf6ef]",children:E.label}),O.jsx("p",{className:"mt-1 text-xs uppercase tracking-[0.22em] text-[#9f9487]",children:E.readiness}),O.jsx("p",{className:"mt-2 text-[11px] uppercase tracking-[0.2em] text-[#78d7d1]",children:b.directReadCount>0?`${b.directReadCount} direct-read method${b.directReadCount===1?"":"s"}`:"sidecar-only route"})]}),O.jsx("span",{className:Ns("rounded-full px-2 py-1 text-[10px] uppercase tracking-[0.24em]",E.stage==="blocked"?"bg-[#59252f] text-[#ffc2d1]":"bg-[#173f36] text-[#aef3db]"),children:E.stage})]}),O.jsx("p",{className:"mt-3 text-sm leading-6 text-[#c5b7a7]",children:E.description})]},E.id)})})]},x.id))})]}),O.jsxs("div",{className:"grid gap-6",children:[O.jsxs("section",{className:"rounded-[28px] border border-white/8 bg-[#15111b] p-5 shadow-[0_18px_50px_rgba(0,0,0,0.32)] lg:p-6",children:[O.jsxs("div",{className:"flex flex-col gap-5 lg:flex-row lg:items-start lg:justify-between",children:[O.jsxs("div",{className:"max-w-3xl",children:[O.jsx("p",{className:"text-[11px] uppercase tracking-[0.28em] text-[#9f9487]",children:m.groupLabel}),O.jsx("h2",{className:"mt-3 text-3xl font-semibold text-[#fbf6ef]",children:m.label}),O.jsx("p",{className:"mt-3 max-w-2xl text-base leading-7 text-[#c5b7a7]",children:m.description}),O.jsx("p",{className:"mt-4 text-sm leading-6 text-[#9f9487]",children:m.groupSummary})]}),O.jsxs("div",{className:"rounded-[24px] border border-white/8 bg-black/20 px-4 py-4 lg:max-w-[320px]",children:[O.jsx("p",{className:"text-[11px] uppercase tracking-[0.28em] text-[#9f9487]",children:"Scenario state"}),O.jsx("p",{className:"mt-3 text-sm leading-6 text-[#f3e7da]",children:y.description}),O.jsx("div",{className:"mt-4 flex flex-wrap gap-2",children:m.scenarios.map(x=>O.jsx("button",{type:"button",onClick:()=>c(x.id),className:Ns("rounded-full border px-3 py-2 text-[11px] uppercase tracking-[0.24em] transition duration-300 focus:outline-none focus-visible:ring-2 focus-visible:ring-[#f49748]",x.id===l?"border-[#f49748]/45 bg-[#f49748]/12 text-[#fff3e1]":"border-white/10 bg-white/[0.02] text-[#b7aa9d] hover:border-white/20 hover:text-[#fbf6ef]"),children:x.label},x.id))})]})]}),O.jsx("div",{className:"mt-6 grid gap-4 md:grid-cols-3",children:m.metrics.map(x=>O.jsxs("article",{className:"rounded-[24px] border border-white/7 bg-white/[0.03] p-4",children:[O.jsx("p",{className:"text-[11px] uppercase tracking-[0.24em] text-[#9f9487]",children:x.label}),O.jsx("p",{className:"mt-3 text-2xl font-semibold text-[#fbf6ef]",children:x.value})]},x.label))})]}),O.jsxs("section",{className:"grid gap-6 xl:grid-cols-[1.25fr_0.75fr]",children:[O.jsxs("article",{className:"rounded-[28px] border border-white/8 bg-[linear-gradient(180deg,_rgba(255,255,255,0.04),_rgba(255,255,255,0.015))] p-5 shadow-[0_18px_50px_rgba(0,0,0,0.32)] lg:p-6",children:[O.jsxs("div",{className:"flex items-center justify-between gap-3",children:[O.jsxs("div",{children:[O.jsx("p",{className:"text-[11px] uppercase tracking-[0.28em] text-[#9f9487]",children:"Route experience"}),O.jsx("h3",{className:"mt-2 text-2xl font-semibold text-[#fbf6ef]",children:v.headline}),O.jsx("p",{className:"mt-3 max-w-3xl text-sm leading-7 text-[#c5b7a7]",children:v.summary})]}),O.jsx("span",{className:"rounded-full border border-[#e8d4aa]/30 bg-[#e8d4aa]/10 px-3 py-1 text-[10px] uppercase tracking-[0.24em] text-[#ffe9b8]",children:v.kicker})]}),O.jsx("div",{className:"mt-6 flex flex-wrap gap-2",children:m.screens.map(x=>O.jsx("button",{type:"button",onClick:()=>s(x.id),className:Ns("rounded-full border px-3 py-2 text-[11px] uppercase tracking-[0.24em] transition duration-300 focus:outline-none focus-visible:ring-2 focus-visible:ring-[#f49748]",x.id===i?"border-[#f49748]/45 bg-[#f49748]/12 text-[#fff3e1]":"border-white/10 bg-white/[0.02] text-[#b7aa9d] hover:border-white/20 hover:text-[#fbf6ef]"),children:x.label},x.id))}),O.jsxs("div",{className:"mt-6 rounded-[24px] border border-[#5bc0be]/20 bg-[#5bc0be]/8 p-4",children:[O.jsxs("div",{className:"flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between",children:[O.jsxs("div",{className:"max-w-2xl",children:[O.jsx("p",{className:"text-[11px] uppercase tracking-[0.28em] text-[#a8f2ec]",children:"Generated route contract"}),O.jsx("h3",{className:"mt-2 text-xl font-semibold text-[#f4fffd]",children:h.routeLabel}),O.jsx("p",{className:"mt-3 text-sm leading-6 text-[#dffaf7]",children:h.rationale})]}),O.jsx("div",{className:"rounded-full border border-[#5bc0be]/35 bg-[#07181b]/50 px-3 py-2 text-[11px] uppercase tracking-[0.24em] text-[#b8fff8]",children:h.enforcementMode})]}),O.jsx("div",{className:"mt-4 flex flex-wrap gap-2",children:h.allowedMethods.length>0?h.allowedMethods.map(x=>O.jsx("span",{className:"rounded-full border border-[#5bc0be]/30 bg-[#0b2225] px-3 py-2 text-[11px] uppercase tracking-[0.18em] text-[#d9fffb]",children:x},x)):O.jsx("span",{className:"rounded-full border border-[#f49748]/35 bg-[#2a170c] px-3 py-2 text-[11px] uppercase tracking-[0.18em] text-[#ffd8b3]",children:"no direct-read rpc methods allowed"})})]}),O.jsx("div",{className:"mt-6 grid gap-4 md:grid-cols-3",children:v.cards.map(x=>O.jsxs("div",{className:Ns("rounded-[24px] border p-4 transition duration-500",x.tone==="alert"?"border-[#6e2e37] bg-[#281218]":x.tone==="warm"?"border-[#6c4c1d] bg-[#24170d]":x.tone==="cool"?"border-[#2d575a] bg-[#0f1f25]":"border-white/8 bg-white/[0.03]"),children:[O.jsx("p",{className:"text-[11px] uppercase tracking-[0.24em] text-[#9f9487]",children:x.label}),O.jsx("p",{className:"mt-3 text-2xl font-semibold text-[#fbf6ef]",children:x.value}),O.jsx("p",{className:"mt-3 text-sm leading-6 text-[#c5b7a7]",children:x.detail})]},x.label))}),O.jsxs("div",{className:"mt-6 grid gap-4 lg:grid-cols-[1.1fr_0.9fr]",children:[O.jsxs("section",{className:"rounded-[24px] border border-white/7 bg-[#0d0a12] p-4",children:[O.jsx("p",{className:"text-[11px] uppercase tracking-[0.24em] text-[#9f9487]",children:"Module stack"}),O.jsx("div",{className:"mt-4 space-y-3",children:v.modules.map(x=>O.jsxs("article",{className:"rounded-[18px] border border-white/7 bg-white/[0.03] p-4",children:[O.jsxs("div",{className:"flex items-start justify-between gap-3",children:[O.jsx("p",{className:"text-sm font-medium text-[#fbf6ef]",children:x.name}),O.jsx("span",{className:"text-[10px] uppercase tracking-[0.24em] text-[#9f9487]",children:x.status})]}),O.jsx("p",{className:"mt-2 text-sm leading-6 text-[#bcae9f]",children:x.detail})]},x.name))})]}),O.jsxs("section",{className:"rounded-[24px] border border-white/7 bg-[#0d0a12] p-4",children:[O.jsx("p",{className:"text-[11px] uppercase tracking-[0.24em] text-[#9f9487]",children:"Journey ladder"}),O.jsx("div",{className:"mt-4 space-y-3",children:m.journey.map((x,E)=>O.jsxs("button",{type:"button",onClick:()=>p(E),className:Ns("w-full rounded-[18px] border px-4 py-4 text-left transition duration-300 focus:outline-none focus-visible:ring-2 focus-visible:ring-[#f49748]",E===d?"border-[#f49748]/35 bg-[#f49748]/10":"border-white/7 bg-white/[0.03] hover:border-white/18"),children:[O.jsxs("div",{className:"flex items-center justify-between gap-3",children:[O.jsx("p",{className:"text-sm font-medium text-[#fbf6ef]",children:x.title}),O.jsx("span",{className:"text-[10px] uppercase tracking-[0.24em] text-[#9f9487]",children:x.state})]}),O.jsx("p",{className:"mt-2 text-sm leading-6 text-[#bcae9f]",children:x.detail})]},x.title))}),O.jsxs("div",{className:"mt-4 rounded-[18px] border border-[#5bc0be]/20 bg-[#5bc0be]/8 p-4",children:[O.jsx("p",{className:"text-[11px] uppercase tracking-[0.24em] text-[#a8f2ec]",children:"Focused transition"}),O.jsx("p",{className:"mt-2 text-sm leading-6 text-[#e5fffc]",children:g.detail})]})]})]})]}),O.jsxs("div",{className:"grid gap-6",children:[O.jsxs("article",{className:"rounded-[28px] border border-white/8 bg-[#15111b] p-5 shadow-[0_18px_50px_rgba(0,0,0,0.32)] lg:p-6",children:[O.jsx("p",{className:"text-[11px] uppercase tracking-[0.28em] text-[#9f9487]",children:"Scenario overlay"}),O.jsx("h3",{className:"mt-2 text-2xl font-semibold text-[#fbf6ef]",children:y.label}),O.jsx("p",{className:"mt-3 text-sm leading-7 text-[#c5b7a7]",children:"This local state changes copy emphasis, highlighted modules, and the journey focus without touching any network call."}),O.jsxs("div",{className:"mt-5 rounded-[24px] border border-white/7 bg-white/[0.03] p-4",children:[O.jsx("p",{className:"text-[11px] uppercase tracking-[0.24em] text-[#9f9487]",children:"Current state"}),O.jsx("p",{className:"mt-2 text-lg font-medium text-[#fbf6ef]",children:y.state}),O.jsx("p",{className:"mt-3 text-sm leading-6 text-[#bcae9f]",children:y.description})]})]}),O.jsxs("article",{className:"rounded-[28px] border border-white/8 bg-[#15111b] p-5 shadow-[0_18px_50px_rgba(0,0,0,0.32)] lg:p-6",children:[O.jsx("p",{className:"text-[11px] uppercase tracking-[0.28em] text-[#9f9487]",children:"Isolation contract"}),O.jsx("h3",{className:"mt-2 text-2xl font-semibold text-[#fbf6ef]",children:"Hard boundaries for this shell"}),O.jsx("div",{className:"mt-5 space-y-3",children:OA.map(x=>O.jsx("div",{className:"rounded-[22px] border border-white/7 bg-white/[0.03] p-4 text-sm leading-6 text-[#c5b7a7]",children:x},x))}),O.jsx("div",{className:"mt-6 space-y-3",children:PA.map(x=>O.jsxs("div",{className:"rounded-[18px] border border-white/7 bg-black/20 p-4",children:[O.jsxs("div",{className:"flex items-center justify-between gap-3",children:[O.jsx("p",{className:"text-sm font-medium text-[#fff5ec]",children:x.title}),O.jsx("span",{className:"text-[10px] uppercase tracking-[0.24em] text-[#9f9487]",children:x.state})]}),O.jsx("p",{className:"mt-2 text-sm leading-6 text-[#bcae9f]",children:x.description})]},x.title))}),O.jsxs("div",{className:"mt-6 rounded-[24px] border border-[#5bc0be]/20 bg-[#5bc0be]/8 p-4",children:[O.jsx("p",{className:"text-[11px] uppercase tracking-[0.28em] text-[#a8f2ec]",children:"How to open"}),O.jsxs("p",{className:"mt-3 text-sm leading-6 text-[#e5fffc]",children:["Run the Vite app and open the shell with ",O.jsx("span",{className:"font-semibold text-[#fff7ef]",children:"?surface=shell"}),". The default route remains untouched."]})]})]})]})]})]})]})]})})}const kA="modulepreload",XA=function(o){return"/"+o},Y_={},Z_=function(e,i,s){let l=Promise.resolve();if(i&&i.length>0){let d=function(h){return Promise.all(h.map(v=>Promise.resolve(v).then(y=>({status:"fulfilled",value:y}),y=>({status:"rejected",reason:y}))))};document.getElementsByTagName("link");const p=document.querySelector("meta[property=csp-nonce]"),m=(p==null?void 0:p.nonce)||(p==null?void 0:p.getAttribute("nonce"));l=d(i.map(h=>{if(h=XA(h),h in Y_)return;Y_[h]=!0;const v=h.endsWith(".css"),y=v?'[rel="stylesheet"]':"";if(document.querySelector(`link[href="${h}"]${y}`))return;const g=document.createElement("link");if(g.rel=v?"stylesheet":kA,v||(g.as="script"),g.crossOrigin="",g.href=h,m&&g.setAttribute("nonce",m),document.head.appendChild(g),v)return new Promise((x,E)=>{g.addEventListener("load",x),g.addEventListener("error",()=>E(new Error(`Unable to preload CSS for ${h}`)))})}))}function c(d){const p=new Event("vite:preloadError",{cancelable:!0});if(p.payload=d,window.dispatchEvent(p),!p.defaultPrevented)throw d}return l.then(d=>{for(const p of d||[])p.status==="rejected"&&c(p.reason);return e().catch(c)})};function jA(o=5e3){const[e,i]=Wt.useState({validators:0,tps:0,blockHeight:0,tvlUsd:0,recentTransactions:[]}),[s,l]=Wt.useState(!0),[c,d]=Wt.useState(null),p=h=>{if(!h||h.length===0)return 0;const v=h.reduce((y,g)=>y+(g.tps||0),0);return Math.round(v/h.length)},m=Wt.useCallback(async()=>{var h,v,y,g,x,E,w,b,S,C,U,N,V,H,F,T;try{const D=await fetch("/api/site/dashboard");if(D.ok){const le=await D.json();i({validators:((w=(E=le.networkTelemetry)==null?void 0:E.validators)==null?void 0:w.length)||42,tps:p((b=le.networkTelemetry)==null?void 0:b.validators),blockHeight:((S=le.dashboard)==null?void 0:S.blockNumber)||1847341,tvlUsd:((C=le.staking)==null?void 0:C.totalValueLockedUsd)||482e5,recentTransactions:((U=le.marketWhales)==null?void 0:U.events)||[]})}else{const le=await Z_(()=>import("./business-store-CXoBgesy.js"),[]);i({validators:((v=(h=le.networkTelemetry)==null?void 0:h.validators)==null?void 0:v.length)||42,tps:p((y=le.networkTelemetry)==null?void 0:y.validators),blockHeight:1847341,tvlUsd:((g=le.staking)==null?void 0:g.totalValueLockedUsd)||482e5,recentTransactions:((x=le.marketWhales)==null?void 0:x.events)||[]})}d(null)}catch(D){try{const le=await Z_(()=>import("./business-store-CXoBgesy.js"),[]);i({validators:((V=(N=le.networkTelemetry)==null?void 0:N.validators)==null?void 0:V.length)||42,tps:p((H=le.networkTelemetry)==null?void 0:H.validators),blockHeight:1847341,tvlUsd:((F=le.staking)==null?void 0:F.totalValueLockedUsd)||482e5,recentTransactions:((T=le.marketWhales)==null?void 0:T.events)||[]}),d(null)}catch{d(D.message)}}finally{l(!1)}},[]);return Wt.useEffect(()=>{m();const h=setInterval(m,o);return()=>clearInterval(h)},[m,o]),{...e,loading:s,error:c,refresh:m}}function WA({value:o,duration:e=2e3,prefix:i="",suffix:s=""}){const[l,c]=Wt.useState(0),d=Wt.useRef(null),p=Wt.useRef(0);Wt.useEffect(()=>{const h=p.current,v=o,y=performance.now(),g=x=>{const E=x-y,w=Math.min(E/e,1),b=1-Math.pow(1-w,3),S=Math.floor(h+(v-h)*b);c(S),w<1?d.current=requestAnimationFrame(g):p.current=v};return d.current&&cancelAnimationFrame(d.current),d.current=requestAnimationFrame(g),()=>{d.current&&cancelAnimationFrame(d.current)}},[o,e]);const m=h=>h>=1e9?(h/1e9).toFixed(2)+"B":h>=1e6?(h/1e6).toFixed(2)+"M":h>=1e3?h.toLocaleString():h.toString();return O.jsxs("span",{className:"animated-counter",children:[i,O.jsx("span",{className:"counter-value",children:m(l)}),s]})}function Hc({label:o,value:e,prefix:i="",suffix:s="",trend:l,loading:c,error:d}){return c?O.jsxs("div",{className:"stat-card bg-surface-dark rounded-2xl p-6 animate-pulse",children:[O.jsx("div",{className:"h-3 bg-gray-700 rounded w-24 mb-4"}),O.jsx("div",{className:"h-10 bg-gray-700 rounded w-32 mb-2"}),O.jsx("div",{className:"h-3 bg-gray-700 rounded w-16"})]}):d?O.jsxs("div",{className:"stat-card bg-surface-dark rounded-2xl p-6 border border-red-500/30",children:[O.jsx("p",{className:"text-xs uppercase tracking-widest opacity-60 mb-2",children:o}),O.jsx("p",{className:"text-red-400 text-sm",children:d})]}):O.jsxs("div",{className:"stat-card bg-surface-dark rounded-2xl p-6 border border-white/5 hover:border-cyan/20 transition-colors duration-300",children:[O.jsx("p",{className:"text-xs uppercase tracking-widest opacity-60 mb-3 font-mono",children:o}),O.jsx("p",{className:"display gradient-text text-4xl lg:text-5xl font-bold mb-2",children:O.jsx(WA,{value:e,prefix:i,suffix:s,duration:1500})}),l!=null&&O.jsxs("div",{className:`flex items-center gap-1 text-sm mt-2 ${l>0?"text-emerald-400":l<0?"text-red-400":"text-gray-400"}`,children:[O.jsx("span",{className:"text-lg",children:l>0?"↑":l<0?"↓":"→"}),O.jsxs("span",{className:"font-mono",children:[Math.abs(l).toFixed(1),"%"]}),O.jsx("span",{className:"opacity-60 text-xs ml-1",children:"24h"})]})]})}function qA({transactions:o=[],loading:e}){const[i,s]=Wt.useState([]);Wt.useEffect(()=>{s(o.slice(0,5))},[o]);const l=m=>{if(!m)return"unknown";try{const h=new Date(m),y=new Date-h,g=Math.floor(y/6e4),x=Math.floor(y/36e5);return g<1?"just now":g<60?`${g}m ago`:x<24?`${x}h ago`:h.toLocaleDateString()}catch{return"unknown"}},c=m=>{switch(m){case"BUY":return"text-emerald-400 bg-emerald-400/10";case"SELL":return"text-red-400 bg-red-400/10";case"STAKE":return"text-cyan bg-cyan/10";case"VOTE":return"text-purple-400 bg-purple-400/10";case"MOVE":return"text-amber-400 bg-amber-400/10";default:return"text-gray-400 bg-gray-400/10"}},d=m=>m?m>=1e6?`$${(m/1e6).toFixed(2)}M`:m>=1e3?`$${(m/1e3).toFixed(1)}K`:`$${m.toFixed(2)}`:"$0",p=m=>m?m.length<=12?m:`${m.slice(0,6)}...${m.slice(-4)}`:"unknown";return e?O.jsxs("div",{className:"live-transaction-feed bg-surface-dark rounded-2xl p-6 border border-white/5",children:[O.jsx("div",{className:"h-4 bg-gray-700 rounded w-40 mb-6 animate-pulse"}),[1,2,3,4,5].map(m=>O.jsxs("div",{className:"flex items-center gap-4 py-3 border-b border-white/5 animate-pulse",children:[O.jsx("div",{className:"h-8 w-8 bg-gray-700 rounded-full"}),O.jsxs("div",{className:"flex-1",children:[O.jsx("div",{className:"h-3 bg-gray-700 rounded w-24 mb-2"}),O.jsx("div",{className:"h-2 bg-gray-700 rounded w-32"})]}),O.jsx("div",{className:"h-3 bg-gray-700 rounded w-16"})]},m))]}):!o||o.length===0?O.jsxs("div",{className:"live-transaction-feed bg-surface-dark rounded-2xl p-6 border border-white/5",children:[O.jsx("h3",{className:"text-sm uppercase tracking-widest opacity-60 mb-4 font-mono",children:"Recent Transactions"}),O.jsxs("div",{className:"text-center py-8",children:[O.jsx("p",{className:"text-gray-400 text-sm",children:"No recent transactions"}),O.jsx("p",{className:"text-gray-500 text-xs mt-1",children:"Transactions will appear here in real-time"})]})]}):O.jsxs("div",{className:"live-transaction-feed bg-surface-dark rounded-2xl p-6 border border-white/5",children:[O.jsxs("div",{className:"flex items-center justify-between mb-6",children:[O.jsx("h3",{className:"text-sm uppercase tracking-widest opacity-60 font-mono",children:"Recent Transactions"}),O.jsxs("span",{className:"flex items-center gap-2 text-xs text-emerald-400",children:[O.jsx("span",{className:"w-2 h-2 bg-emerald-400 rounded-full animate-pulse"}),"Live"]})]}),O.jsx("div",{className:"space-y-1",children:i.map((m,h)=>{var v;return O.jsxs("div",{className:"flex items-center gap-4 py-3 border-b border-white/5 last:border-0 hover:bg-white/5 rounded-lg px-2 transition-colors duration-200",children:[O.jsx("div",{className:`px-2 py-1 rounded text-xs font-mono font-bold ${c(m.type)}`,children:m.type}),O.jsxs("div",{className:"flex-1 min-w-0",children:[O.jsxs("div",{className:"flex items-center gap-2",children:[O.jsx("span",{className:"text-sm font-mono text-white/80",children:p(m.wallet)}),O.jsx("span",{className:"text-xs text-gray-500",children:l(m.timestamp)})]}),O.jsx("p",{className:"text-xs text-gray-400 truncate mt-1",children:m.detail||"Transaction"})]}),O.jsxs("div",{className:"text-right",children:[O.jsxs("p",{className:"text-sm font-mono text-white",children:[((v=m.amountX3S)==null?void 0:v.toLocaleString())||0," X3S"]}),O.jsx("p",{className:"text-xs text-gray-400",children:d(m.amountUsd)})]})]},m.id||h)})}),o.length>5&&O.jsx("div",{className:"mt-4 pt-4 border-t border-white/5 text-center",children:O.jsxs("button",{className:"text-xs text-cyan hover:text-cyan/80 transition-colors font-mono",children:["View all ",o.length," transactions →"]})})]})}function YA(){const{validators:o,tps:e,blockHeight:i,tvlUsd:s,recentTransactions:l,loading:c,error:d,refresh:p}=jA(5e3);return O.jsxs("section",{className:"live-stats-dashboard section py-20 px-6",id:"stats",children:[O.jsx(Gh,{text:"Live Network"}),O.jsx("h2",{className:"section-title text-4xl lg:text-6xl font-bold mb-4",children:"X3 Chain Statistics"}),O.jsxs("p",{className:"section-sub max-w-2xl mx-auto text-gray-400 mb-12",children:["Real-time network health, validator activity, and transaction flow.",O.jsx("span",{className:"text-cyan ml-2",children:"Auto-refreshes every 5 seconds."})]}),d&&O.jsxs("div",{className:"max-w-4xl mx-auto mb-8 p-4 bg-red-500/10 border border-red-500/30 rounded-xl flex items-center justify-between",children:[O.jsxs("div",{className:"flex items-center gap-3",children:[O.jsx("span",{className:"text-red-400",children:"⚠️"}),O.jsx("span",{className:"text-red-300 text-sm",children:d})]}),O.jsx(Qc,{outline:!0,onClick:p,children:"Retry"})]}),O.jsxs("div",{className:"max-w-6xl mx-auto grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-12",children:[O.jsx(Hc,{label:"Active Validators",value:o,loading:c,error:d&&!o?"Failed to load":null}),O.jsx(Hc,{label:"Avg TPS",value:e,suffix:" tx/s",loading:c,error:d&&!e?"Failed to load":null}),O.jsx(Hc,{label:"Block Height",value:i,prefix:"#",loading:c,error:d&&!i?"Failed to load":null}),O.jsx(Hc,{label:"Total Value Locked",value:s,prefix:"$",loading:c,error:d&&!s?"Failed to load":null})]}),O.jsxs("div",{className:"max-w-6xl mx-auto",children:[O.jsxs("div",{className:"flex items-center justify-between mb-6",children:[O.jsx("h3",{className:"text-lg font-mono text-white/80",children:"Network Activity"}),O.jsx(Qc,{outline:!0,onClick:p,disabled:c,children:c?"Refreshing...":"↻ Refresh Now"})]}),O.jsxs("div",{className:"grid grid-cols-1 lg:grid-cols-2 gap-6",children:[O.jsx(qA,{transactions:l,loading:c}),O.jsxs("div",{className:"bg-surface-dark rounded-2xl p-6 border border-white/5",children:[O.jsx("h3",{className:"text-sm uppercase tracking-widest opacity-60 mb-6 font-mono",children:"Network Status"}),O.jsxs("div",{className:"space-y-4",children:[O.jsxs("div",{className:"flex items-center justify-between p-3 bg-white/5 rounded-lg",children:[O.jsx("span",{className:"text-sm text-gray-400",children:"Network Status"}),O.jsxs("span",{className:"flex items-center gap-2 text-emerald-400",children:[O.jsx("span",{className:"w-2 h-2 bg-emerald-400 rounded-full animate-pulse"}),"Operational"]})]}),O.jsxs("div",{className:"flex items-center justify-between p-3 bg-white/5 rounded-lg",children:[O.jsx("span",{className:"text-sm text-gray-400",children:"Avg Finality"}),O.jsx("span",{className:"font-mono text-white",children:"0.4s"})]}),O.jsxs("div",{className:"flex items-center justify-between p-3 bg-white/5 rounded-lg",children:[O.jsx("span",{className:"text-sm text-gray-400",children:"Network Uptime"}),O.jsx("span",{className:"font-mono text-emerald-400",children:"99.8%"})]}),O.jsxs("div",{className:"flex items-center justify-between p-3 bg-white/5 rounded-lg",children:[O.jsx("span",{className:"text-sm text-gray-400",children:"Avg Tx Fee"}),O.jsx("span",{className:"font-mono text-cyan",children:"$0.0001"})]}),O.jsx("div",{className:"pt-4 border-t border-white/5",children:O.jsxs("p",{className:"text-xs text-gray-500 text-center",children:["Last updated: ",new Date().toLocaleTimeString()]})})]})]})]})]})]})}function ZA(){return typeof window<"u"&&new URLSearchParams(window.location.search).get("surface")==="shell"?O.jsx(VA,{}):O.jsxs(O.Fragment,{children:[O.jsx(UA,{}),O.jsx(YA,{})]})}Qy.createRoot(document.getElementById("root")).render(O.jsx(ky.StrictMode,{children:O.jsx(ZA,{})}));
