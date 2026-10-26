(function () {
  'use strict';
  const g = globalThis, ns = g[Symbol.for('ts')], native = ns.makeNativeFunction;
  const bridge = g.__formNative;
  delete g.__formNative;
  const readValue = ns.formControlValue;
  delete ns.formControlValue;
  const checkedValue=ns.formControlChecked;delete ns.formControlChecked;
  const dialogSubmission=ns.dialogFormSubmission;delete ns.dialogFormSubmission;
  const selectedOptions=ns.formSelectedOptions,resetControl=ns.formResetControl;delete ns.formSelectedOptions;delete ns.formResetControl;
  let fileEntries=null;ns.registerFileEntries=entries=>{fileEntries=entries;delete ns.registerFileEntries;};
  const apply = Reflect.apply, define = Object.defineProperty, create = Object.create;
  const string = String, number = Number;
  const ErrorType = TypeError, Exception = DOMException, Regex = RegExp;
  const isNaNValue=Number.isNaN,absolute=Math.abs,round=Math.round,floor=Math.floor;
  const tagGetter = ns.domAccessor('tagName', false, 'Element');
  const localNameGetter = ns.domAccessor('localName', false, 'Element');
  const parentGetter = ns.domAccessor('parentElement', false, 'Node');
  const connectedGetter = ns.domAccessor('isConnected', false, 'Node');
  const getRoot = Node.prototype.getRootNode, query = Element.prototype.querySelectorAll, documentQuery = Document.prototype.querySelectorAll;
  const nodeTypeGetter = ns.domAccessor('nodeType', false, 'Node');
  const collectionRecord=ns.formCollectionFactory.getRecord;
  const fragmentQuery = DocumentFragment.prototype.querySelectorAll;
  // Owner trees may be documents, shadow roots or detached element subtrees.
  const queryAll = (root, selector) => {const kind=apply(nodeTypeGetter,root,[]);const result=apply(kind===9?documentQuery:kind===11?fragmentQuery:query,root,[selector]);const record=collectionRecord(result);return record?record.read():result;};
  const attribute = Element.prototype.getAttribute, contains = Node.prototype.contains;
  const lower = Function.prototype.call.bind(String.prototype.toLowerCase);
  const regexTest = Function.prototype.call.bind(RegExp.prototype.test);
  const associatedGet=element=>bridge('associated',element),associatedSet=(element,record)=>bridge('associated',element,record);
  const privateGet=(object,family)=>bridge('private',object,family),privateSet=(object,family,value)=>bridge('private',object,family,value);
  const customGet=element=>privateGet(element,'customMessage'),customSet=(element,value)=>privateSet(element,'customMessage',value);
  const validityGet=state=>privateGet(state,'validityRecord'),validitySet=(state,value)=>privateSet(state,'validityRecord',value);
  const stateGet=element=>privateGet(element,'validityObject'),stateSet=(element,value)=>privateSet(element,'validityObject',value);
  const submitterGet=event=>privateGet(event,'submitter'),submitterSet=(event,value)=>privateSet(event,'submitter',value),submitterHas=event=>submitterGet(event)!==undefined;
  const EventClass = Event;
  // Shared with later private modules (input_members adds rangeState; element_internals takes it).
  const internals = create(null);
  const tag = element => lower(apply(tagGetter, element, []));
  const attr = (element, name) => apply(attribute, element, [name]);
  const parent = element => apply(parentGetter, element, []);
  const owner = element => bridge('owner', element);
  for(const [interfaceName,localName] of [['HTMLInputElement','input'],['HTMLButtonElement','button'],['HTMLTextAreaElement','textarea'],['HTMLSelectElement','select'],['HTMLFieldSetElement','fieldset'],['HTMLObjectElement','object'],['HTMLOutputElement','output']]){
    const constructor=g[interfaceName];if(typeof constructor!=='function')continue;
    define(constructor.prototype,'form',{configurable:true,enumerable:true,get:native(function(){
      if(!bridge('html-element',this)||apply(localNameGetter,this,[])!==localName)throw new ErrorType('Illegal invocation');
      return owner(this);
    },'get form',0)});
  }
  // label.form is its labeled control's form owner; option.form is its select's (directly or
  // through one optgroup). Neither element is itself form-associated.
  const formAssociated = ['button','fieldset','input','object','output','select','textarea'];
  // form_collections.js adds the native label-control bridge to this shared capture record.
  const selectOf = option => {
    let container = parent(option);
    if (container && bridge('html-element', container) && apply(localNameGetter, container, []) === 'optgroup') container = parent(container);
    return container && bridge('html-element', container) && apply(localNameGetter, container, []) === 'select' ? container : null;
  };
  for (const [interfaceName, localName, read] of [
    ['HTMLLabelElement', 'label', label => { const control = internals.labelControl(label); if (!control) return null; const kind = apply(localNameGetter, control, []); return bridge('html-element', control) && apply(controlIndexOf, formAssociated, [kind]) >= 0 || associatedGet(control) ? owner(control) : null; }],
    ['HTMLOptionElement', 'option', option => { const select = selectOf(option); return select ? owner(select) : null; }],
  ]) {
    define(g[interfaceName].prototype, 'form', {configurable:true, enumerable:true, get:native(function(){
      if(!bridge('html-element',this)||apply(localNameGetter,this,[])!==localName)throw new ErrorType('Illegal invocation');
      return read(this);
    },'get form',0)});
  }
  const present = (element, name) => attr(element, name) !== null;
  const type = element => {
    if(tag(element)!=='button')return lower(attr(element,'type')||'text');
    const value=lower(attr(element,'type')||'');
    // Auto state: a missing/invalid type is submit unless the button is a command invoker.
    return value==='submit'||value==='button'||value==='reset'?value:present(element,'command')||present(element,'commandfor')?'button':'submit';
  };
  const controlIndexOf=Array.prototype.indexOf;
  const controlTags = ['input','textarea','select','button','fieldset','object','output'];
  function checkControl(element) {
    const kind = tag(element);
    if(associatedGet(element))return "custom";
    for (let index = 0; index < controlTags.length; index++) if (kind === controlTags[index]) return kind;
    throw new ErrorType('Illegal invocation');
  }
  function checkForm(form) {
    if (tag(form) !== 'form') throw new ErrorType('Illegal invocation');
    return form;
  }
  function disabled(element) {
    if (present(element, 'disabled')) return true;
    let ancestor = parent(element);
    while (ancestor) {
      if (tag(ancestor) === 'fieldset' && present(ancestor, 'disabled')) {
        const legends = apply(query, ancestor, ['legend']);
        let first = null;
        for (let index = 0; index < legends.length; index++) if (parent(legends[index]) === ancestor) {first = legends[index];break;}
        if (!first || !apply(contains, first, [element])) return true;
      }
      ancestor = parent(ancestor);
    }
    return false;
  }
  function candidate(element) {
    const kind = checkControl(element), inputType = type(element);
    if(kind==="custom")return !disabled(element)&&!present(element,"readonly");
    if (kind === 'fieldset' || kind === 'object' || kind === 'output' || disabled(element)) return false;
    if (kind === 'input' && (inputType === 'hidden' || inputType === 'reset' || inputType === 'button' || inputType === 'image')) return false;
    if (kind === 'button' && inputType !== 'submit') return false;
    if (present(element, 'readonly') && (kind === 'textarea' || kind === 'input' && regexTest(/^(text|search|url|tel|email|password|date|month|week|time|datetime-local|number)$/, inputType))) return false;
    let ancestor = parent(element);
    while (ancestor) {if (tag(ancestor) === 'datalist') return false;ancestor = parent(ancestor);}
    return true;
  }
  function formControls(form) {
    const root = apply(getRoot, form, []), nodes = queryAll(root, '*');
    const result = [];
    for (let index = 0; index < nodes.length; index++) if ((associatedGet(nodes[index])||apply(controlIndexOf,controlTags,[tag(nodes[index])])>=0)&&owner(nodes[index]) === form) define(result,result.length,{__proto__:null,value:nodes[index],writable:true,enumerable:true,configurable:true});
    return result;
  }
  function flags(element) {
    const associatedRecord=associatedGet(element);if(associatedRecord)return associatedRecord.flags;
    const kind = checkControl(element), inputType = type(element);
    const value = readValue(element), required = present(element, 'required');
    // Like Blink, the flags describe the value whether or not the control is barred from
    // validation; only valueMissing needs a mutable (enabled, not read-only) control.
    const readOnly = present(element, 'readonly') && (kind === 'textarea' || kind === 'input' && regexTest(/^(text|search|url|tel|email|password|date|month|week|time|datetime-local|number)$/, inputType));
    const mutable = !disabled(element) && !readOnly;
    const state = {valueMissing:false,typeMismatch:false,patternMismatch:false,tooLong:false,tooShort:false,rangeUnderflow:false,rangeOverflow:false,stepMismatch:false,badInput:false,customError:!!customGet(element)};
    {
      if (kind === 'input' && inputType === 'radio') {
        // A radio group is required, and missing a value, as a whole: any member's required
        // attribute applies to all of them. An unnamed radio is its own group.
        const name = attr(element,'name');
        let groupRequired = required, checked = checkedValue(element);
        if (name) {
          const root = apply(getRoot, element, []), radios = queryAll(root,'input');
          for (let index = 0; index < radios.length; index++) {
            const radio = radios[index];
            if (type(radio) !== 'radio' || attr(radio,'name') !== name || owner(radio) !== owner(element)) continue;
            if (present(radio,'required')) groupRequired = true;
            if (checkedValue(radio)) checked = true;
          }
        }
        state.valueMissing = mutable && groupRequired && !checked;
      } else if (mutable && required && (kind === 'textarea' || kind === 'select' || kind === 'input' && !regexTest(/^(range|color|hidden|button|submit|reset|image)$/,inputType))) {
        if (inputType === 'checkbox') state.valueMissing = !checkedValue(element);
        else state.valueMissing = value === '';
      }
      if (kind === 'input' && value) {
        if (inputType === 'email') state.typeMismatch = !regexTest(/^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$/,value);
        if(inputType==='email'&&present(element,'multiple')) {
          const parts=value.split(',');state.typeMismatch=false;
          for(let index=0;index<parts.length;index++)if(!regexTest(/^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$/,parts[index].trim()))state.typeMismatch=true;
        }
        if (inputType === 'url') {try {if(!bridge('absolute-url',element,value))throw new ErrorType('Invalid URL');} catch (_) {state.typeMismatch=true;}}
        const pattern = attr(element, 'pattern');
        if (pattern !== null && regexTest(/^(text|search|url|tel|email|password)$/,inputType)) {
          try {state.patternMismatch=!regexTest(new Regex('^(?:'+pattern+')$','v'),value);} catch (_) {}
        }
        if (inputType === 'number') {
          const numeric = number(value), minimum = attr(element,'min'), maximum=attr(element,'max'), step=attr(element,'step');
          if (minimum !== null && minimum !== '' && !isNaNValue(number(minimum))) state.rangeUnderflow=numeric<number(minimum);
          if (maximum !== null && maximum !== '' && !isNaNValue(number(maximum))) state.rangeOverflow=numeric>number(maximum);
          if (step !== 'any') {
            const interval=step!==null&&number(step)>0?number(step):1;
            const basis=minimum!==null&&minimum!==''&&!isNaNValue(number(minimum))?number(minimum):number(attr(element,'value')||0);
            const difference=(numeric-basis)/interval;
            state.stepMismatch=absolute(difference-round(difference))>1e-7;
          }
        } else if (internals.rangeState && regexTest(/^(range|date|month|week|time|datetime-local)$/,inputType)) {
          // Blink's StepRange over the sanitized value (a range value is clamped into range).
          const range=internals.rangeState(element);
          if (range) {state.rangeUnderflow=range.underflow;state.rangeOverflow=range.overflow;state.stepMismatch=range.mismatch;}
        }
      }
    }
    state.valid=!(state.valueMissing||state.typeMismatch||state.patternMismatch||state.tooLong||state.tooShort||state.rangeUnderflow||state.rangeOverflow||state.stepMismatch||state.badInput||state.customError);
    return state;
  }
  function message(element) {
    checkControl(element);const state=flags(element);
    const associatedRecord=associatedGet(element);if(associatedRecord)return candidate(element)&&!state.valid?associatedRecord.message:"";
    if(!candidate(element)||state.valid)return '';
    const own=customGet(element);if(own)return own;
    const inputType=type(element),value=readValue(element);
    if(state.valueMissing) {
      if(inputType==='checkbox')return 'Please check this box if you want to proceed.';
      if(inputType==='radio')return 'Please select one of these options.';
      if(inputType==='file')return 'Please select a file.';
      return tag(element)==='select'?'Please select an item in the list.':'Please fill out this field.';
    }
    if(state.typeMismatch) {
      if(inputType==='url')return 'Please enter a URL.';
      const at=value.indexOf('@');
      if(at<0)return "Please include an '@' in the email address. '"+value+"' is missing an '@'.";
      if(at===0)return "Please enter a part followed by '@'. '"+value+"' is incomplete.";
      if(at===value.length-1)return "Please enter a part following '@'. '"+value+"' is incomplete.";
      const host=value.slice(at+1);
      if(host[0]==='.'||host[host.length-1]==='.'||host.includes('..'))return "'.' is used at a wrong position in '"+host+"'.";
      return 'Please enter an email address.';
    }
    if(state.patternMismatch)return 'Please match the requested format.';
    if(state.rangeUnderflow)return 'Value must be greater than or equal to '+attr(element,'min')+'.';
    if(state.rangeOverflow)return 'Value must be less than or equal to '+attr(element,'max')+'.';
    if(state.stepMismatch) {
      const minimum=attr(element,'min'),step=attr(element,'step'),interval=step!==null&&number(step)>0?number(step):1;
      const base=minimum!==null&&minimum!==''&&!isNaNValue(number(minimum))?number(minimum):number(attr(element,'value')||0);
      const below=base+floor((number(value)-base)/interval)*interval;
      return 'Please enter a valid value. The two nearest valid values are '+below+' and '+(below+interval)+'.';
    }
    return '';
  }
  function validity(element) {
    checkControl(element);
    let state=stateGet(element);
    if (!state) {state=create(ValidityState.prototype);validitySet(state,element);stateSet(element,state);}
    return state;
  }
  const ValidityState=native(function(){throw new ErrorType('Illegal constructor');},'ValidityState',0,true);
  define(ValidityState.prototype,Symbol.toStringTag,{value:'ValidityState',configurable:true});
  define(ValidityState,'prototype',{writable:false});
  for (const key of ['valueMissing','typeMismatch','patternMismatch','tooLong','tooShort','rangeUnderflow','rangeOverflow','stepMismatch','badInput','customError','valid']) {
    define(ValidityState.prototype,key,{get:native(function(){const element=validityGet(this);if(!element)throw new ErrorType('Illegal invocation');return flags(element)[key];},'get '+key,0),enumerable:true,configurable:true});
  }
  define(g,'ValidityState',{value:ValidityState,writable:true,configurable:true});
  function validateControl(element) {
    if (!candidate(element)||flags(element).valid) return true;
    bridge('emit',element,new EventClass('invalid',{cancelable:true}));
    return false;
  }
  function validate(element) {
    if (tag(element) !== 'form') return validateControl(element);
    const controls=formControls(element);let valid=true;
    for(let index=0;index<controls.length;index++)if(!validateControl(controls[index]))valid=false;
    return valid;
  }
  const submitImplementation=class SubmitEvent extends EventClass {
    constructor(type, init={}) {
      if (!arguments.length) throw new ErrorType('SubmitEvent requires a type');
      super(type,init);
      const submitter=init?.submitter??null;
      if(submitter!==null) {try{tag(submitter);}catch(_){throw new ErrorType('submitter must be an HTMLElement');}}
      submitterSet(this,submitter);
    }
  };
  const SubmitEventClass=native(submitImplementation,'SubmitEvent',1,true,true);
  SubmitEventClass.prototype=submitImplementation.prototype;
  Object.setPrototypeOf(SubmitEventClass,EventClass);
  define(SubmitEventClass.prototype,'constructor',{value:SubmitEventClass,writable:true,configurable:true});
  define(SubmitEventClass,'prototype',{writable:false});
  define(SubmitEventClass.prototype,'submitter',{get:native(function(){if(!submitterHas(this))throw new ErrorType('Illegal invocation');return submitterGet(this);},'get submitter',0),enumerable:true,configurable:true});
  define(SubmitEventClass.prototype,Symbol.toStringTag,{value:'SubmitEvent',configurable:true});
  define(g,'SubmitEvent',{value:SubmitEventClass,writable:true,configurable:true});
  const formDataDocument=g.document;
  const formDataImplementation=class FormDataEvent extends EventClass {
    constructor(type,init) {
      if(!arguments.length)throw new ErrorType('FormDataEvent requires a type');
      super(type,init);
      const value=init?.formData;
      if(value===undefined)throw new ErrorType('FormDataEvent requires formData');
      const snapshot=bridge('value-snapshot',formDataDocument,value);
      if(!snapshot||snapshot.kind!=='formdata')throw new ErrorType('formData must be a genuine FormData');
      privateSet(this,'formData',value);
    }
  };
  const FormDataEventClass=native(formDataImplementation,'FormDataEvent',2,true,true);
  FormDataEventClass.prototype=formDataImplementation.prototype;
  Object.setPrototypeOf(FormDataEventClass,EventClass);
  delete FormDataEventClass.prototype.constructor;
  define(FormDataEventClass.prototype,'formData',{get:native(function(){const value=privateGet(this,'formData');if(value===undefined)throw new ErrorType('Illegal invocation');return value;},'get formData',0),enumerable:true,configurable:true});
  define(FormDataEventClass.prototype,'constructor',{value:FormDataEventClass,writable:true,configurable:true});
  define(FormDataEventClass.prototype,Symbol.toStringTag,{value:'FormDataEvent',configurable:true});
  define(FormDataEventClass,'prototype',{writable:false});
  define(g,'FormDataEvent',{value:FormDataEventClass,writable:true,configurable:true});
  g.__captureFormDataEvent=(form,data)=>bridge('emit',form,new FormDataEventClass('formdata',{bubbles:true,formData:data}));
  const freeze = Object.freeze;
  function submission(form, submitter) {
    const entries=[],charsetEntries=[];
    const controls=formControls(form);
    const append=(name,value)=>{entries[entries.length]=freeze([name,value]);};
    for(let i=0;i<controls.length;i++) {
      const control=controls[i],kind=tag(control),inputType=type(control),name=attr(control,'name');
      const associatedRecord=associatedGet(control);if(associatedRecord){if(!disabled(control)&&associatedRecord.value!==null){const value=associatedRecord.value;if(typeof value==='object'&&value.kind==='formdata'){for(let at=0;at<value.entries.length;at++)append(value.entries[at][0],value.entries[at][1]);}else if(name)append(name,typeof value==='object'&&value.kind==='file'?value.value:value);}continue;}
      if(disabled(control)||!name||kind==='fieldset'||kind==='object'||kind==='output')continue;
      if(kind==='button'||kind==='input'&&regexTest(/^(submit|reset|button|image)$/,inputType)) {
        if(control!==submitter)continue;
        if(inputType==='image'){append(name+'.x','0');append(name+'.y','0');continue;}
      }
      if(kind==='input'&&regexTest(/^(checkbox|radio)$/,inputType)&&!checkedValue(control))continue;
      if(kind==='input'&&inputType==='file'){const files=fileEntries&&fileEntries(control);if(!files)throw new Exception('Native FileList submission is not available.','NotSupportedError');for(let at=0;at<files.length;at++)append(name,files[at]);continue;}
      if(kind==='select') {
        const options=selectedOptions(control);
        for(let j=0;j<options.length;j++) {
          const option=options[j],group=parent(option);
          if(present(option,'disabled')||group&&tag(group)==='optgroup'&&present(group,'disabled'))continue;
          append(name,attr(option,'value')??readValue(option));
        }
      } else {if(inputType==='hidden'&&name==='_charset_')charsetEntries[charsetEntries.length]=entries.length;append(name,inputType==='checkbox'||inputType==='radio'?attr(control,'value')??'on':readValue(control));}
    }
    const override=(field,fallback)=>submitter&&present(submitter,'form'+field)?attr(submitter,'form'+field):attr(form,fallback||field);
    const method=lower(override('method')||'get'),enctype=lower(override('enctype')||'application/x-www-form-urlencoded');
    const result=create(null);
    result.action=override('action');result.method=regexTest(/^(get|post|dialog)$/,method)?method:'get';
    result.enctype=regexTest(/^(application\/x-www-form-urlencoded|multipart\/form-data|text\/plain)$/,enctype)?enctype:'application/x-www-form-urlencoded';
    result.target=override('target')||'';result.acceptCharset=attr(form,'accept-charset');result.entries=freeze(entries);result.charsetEntries=freeze(charsetEntries);
    return freeze(result);
  }
  function queue(form,submitter) {
    if(!apply(connectedGetter,form,[])||privateGet(form,'constructing-entries'))return;
    // method=dialog closes the nearest ancestor dialog before any entry list exists.
    const method=lower(submitter&&present(submitter,'formmethod')?attr(submitter,'formmethod'):attr(form,'method')||'');
    if(method==='dialog'){if(dialogSubmission)dialogSubmission(form,submitter);return;}
    privateSet(form,'constructing-entries',true);
    try {if(bridge('submit',form,submitter,submission(form,submitter))===true)return;throw new Exception('Form navigation requires a configured native submission transport.','NotSupportedError');}
    finally {privateSet(form,'constructing-entries',false);}
  }
  function normalizeSubmitter(form,submitter) {
    if(submitter!==null&&submitter!==undefined) {
      const kind=tag(submitter), inputType=type(submitter);
      if(!((kind==='button'&&inputType==='submit')||(kind==='input'&&(inputType==='submit'||inputType==='image'))))throw new ErrorType('The specified element is not a submit button.');
      if(owner(submitter)!==form)throw new Exception('The specified element is not owned by this form element.','NotFoundError');
    } else submitter=null;
    return submitter;
  }
  function request(form, submitter) {
    checkForm(form);submitter=normalizeSubmitter(form,submitter);
    if(!apply(connectedGetter,form,[])||!bridge('allowed',form)||!bridge('active',form)||privateGet(form,'firing-submit'))return;
    privateSet(form,'firing-submit',true);
    try {
      // A submission attempt sets every listed input, select and textarea's user validity.
      const controls=formControls(form);
      for(let i=0;i<controls.length;i++){const kind=tag(controls[i]);if(kind==='input'||kind==='select'||kind==='textarea')privateSet(controls[i],'userValidity',true);}
      if(!present(form,'novalidate')&&!(submitter&&present(submitter,'formnovalidate'))&&!validate(form))return;
      const event=new SubmitEventClass('submit',{bubbles:true,cancelable:true,composed:submitter!==null,submitter});
      if(!bridge('emit',form,event))return;
    } finally {privateSet(form,'firing-submit',false);}
    queue(form,submitter);
  }
  function method(proto,name,fn,length) {define(proto,name,{value:native(fn,name,length),enumerable:true,writable:true,configurable:true});}
  for(const name of ['HTMLInputElement','HTMLTextAreaElement','HTMLSelectElement','HTMLButtonElement','HTMLFieldSetElement','HTMLObjectElement','HTMLOutputElement']) {
    const proto=g[name].prototype;
    const expected=({HTMLInputElement:'input',HTMLTextAreaElement:'textarea',HTMLSelectElement:'select',HTMLButtonElement:'button',HTMLFieldSetElement:'fieldset',HTMLObjectElement:'object',HTMLOutputElement:'output'})[name];
    const checkInterface=element=>{if(tag(element)!==expected)throw new ErrorType('Illegal invocation');};
    define(proto,'willValidate',{get:native(function(){checkInterface(this);return candidate(this);},'get willValidate',0),enumerable:true,configurable:true});
    define(proto,'validity',{get:native(function(){checkInterface(this);return validity(this);},'get validity',0),enumerable:true,configurable:true});
    define(proto,'validationMessage',{get:native(function(){checkInterface(this);return message(this);},'get validationMessage',0),enumerable:true,configurable:true});
    method(proto,'setCustomValidity',function(value){checkInterface(this);if(!arguments.length)throw new ErrorType('setCustomValidity requires 1 argument');if(typeof value==='symbol')throw new ErrorType('Cannot convert a Symbol to a string');customSet(this,string(value));},1);
    for(const methodName of ['checkValidity','reportValidity'])method(proto,methodName,function(){checkInterface(this);return validate(this);},0);
  }
  const formPrototype=HTMLFormElement.prototype;
  const reflectedSetAttribute=Element.prototype.setAttribute;
  const wellFormed=String.prototype.toWellFormed;
  for(const [property,attributeName,choices,fallback] of [
    ['action','action',null,null],['method','method',['get','post','dialog'],'get'],
    ['enctype','enctype',['application/x-www-form-urlencoded','multipart/form-data','text/plain'],'application/x-www-form-urlencoded'],
    ['encoding','enctype',['application/x-www-form-urlencoded','multipart/form-data','text/plain'],'application/x-www-form-urlencoded'],
    ['target','target',null,''],['name','name',null,''],['acceptCharset','accept-charset',null,''],
    ['autocomplete','autocomplete',['on','off'],'on'],['rel','rel',null,'']]) {
    define(formPrototype,property,{
      get:native(function(){checkForm(this);if(property==='action')return bridge('action-url',this);const value=attr(this,attributeName)||'';if(!choices)return value;const normalized=lower(value);return choices.indexOf(normalized)>=0?normalized:fallback;},'get '+property,0),
      set:native(function(value){checkForm(this);if(typeof value==='symbol')throw new ErrorType('Cannot convert a Symbol to a string');value=string(value);if(property==='action')value=apply(wellFormed,value,[]);apply(reflectedSetAttribute,this,[attributeName,value]);},'set '+property,1),enumerable:true,configurable:true
    });
  }

  for(const [interfaceName,kind] of [['HTMLButtonElement','button'],['HTMLInputElement','input']]) {
    const prototype=g[interfaceName].prototype;
    const check=element=>{if(!bridge('html-element',element)||apply(localNameGetter,element,[])!==kind)throw new ErrorType('Illegal invocation');};
    for(const [property,attributeName,choices,fallback] of [
      ['formAction','formaction',null,null],
      ['formEnctype','formenctype',['application/x-www-form-urlencoded','multipart/form-data','text/plain'],'application/x-www-form-urlencoded'],
      ['formMethod','formmethod',['get','post','dialog'],'get'],['formTarget','formtarget',null,'']]) {
      define(prototype,property,{
        get:native(function(){check(this);if(property==='formAction')return bridge('submitter-action-url',this);const value=attr(this,attributeName);if(value===null)return '';if(!choices)return value;const normalized=lower(value);return choices.indexOf(normalized)>=0?normalized:fallback;},'get '+property,0),
        set:native(function(value){check(this);if(typeof value==='symbol')throw new ErrorType('Cannot convert a Symbol to a string');value=string(value);if(property==='formAction'&&kind==='button')value=apply(wellFormed,value,[]);apply(reflectedSetAttribute,this,[attributeName,value]);},'set '+property,1),enumerable:true,configurable:true
      });
    }
  }

  for(const methodName of ['checkValidity','reportValidity'])method(formPrototype,methodName,function(){checkForm(this);return validate(this);},0);
  method(formPrototype,'reset',function(){checkForm(this);const event=new EventClass('reset',{bubbles:true,cancelable:true});if(!bridge('emit',this,event))return;const controls=formControls(this);for(let i=0;i<controls.length;i++){privateSet(controls[i],'userValidity',false);const record=associatedGet(controls[i]);if(record){if(record.reset)record.reset();}else resetControl(controls[i]);}},0);
  method(formPrototype,'requestSubmit',function(submitter){return request(this,submitter);},0);
  method(formPrototype,'submit',function(){checkForm(this);if(!bridge('allowed',this)||!bridge('active',this))return;return queue(this,null);},0);
  for(const [proto,property,attributeName] of [[formPrototype,'noValidate','novalidate'],[HTMLButtonElement.prototype,'formNoValidate','formnovalidate'],[HTMLInputElement.prototype,'formNoValidate','formnovalidate']]) {
    const setAttribute=Element.prototype.setAttribute,removeAttribute=Element.prototype.removeAttribute;
    define(proto,property,{get:native(function(){return present(this,attributeName);},'get '+property,0),set:native(function(value){if(value)apply(setAttribute,this,[attributeName,'']);else apply(removeAttribute,this,[attributeName]);},'set '+property,1),enumerable:true,configurable:true});
  }
  g.__captureFormEntries=(form,submitter,hold=false)=>{
    if(form===undefined){if(submitter!==undefined&&submitter!==null&&!bridge('html-element',submitter))throw new ErrorType('The submitter is not an HTMLElement.');return freeze([]);}
    checkForm(form);submitter=normalizeSubmitter(form,submitter);
    if(privateGet(form,'constructing-entries'))throw new Exception('The form entry list is already being constructed.','InvalidStateError');
    privateSet(form,'constructing-entries',true);let complete=false;
    try {
      const payload=submission(form,submitter),entries=[];
      for(let i=0;i<payload.entries.length;i++)entries[i]=payload.entries[i];
      for(let i=0;i<payload.charsetEntries.length;i++){const at=payload.charsetEntries[i];entries[at]=freeze([entries[at][0],'UTF-8']);}
      complete=true;return freeze(entries);
    } finally {if(!hold||!complete)privateSet(form,'constructing-entries',false);}
  };
  g.__captureFormCollections={bridge,privateGet,privateSet,formControls,checkForm,attr,tag,type,readValue,checkedValue,queryAll,getRoot,owner,associatedGet,localNameGetter};
  g.__captureFormInternals = Object.assign(internals, {
    register:(host,record)=>associatedSet(host,record),owner,disabled,candidate,validity,message,validateControl,
    attribute:attr,parent,tag,queryAll,getRoot,apply,emit:(host,event)=>bridge('emit',host,event),
    flags,type,formControls,readValue,checkedValue,associatedGet,userValidity:(element)=>privateGet(element,'userValidity')===true,
  });
  g.__captureFormSubmission=request;
})();
