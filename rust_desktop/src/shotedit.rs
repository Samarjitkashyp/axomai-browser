//! Screenshot editor: arrows, highlights, blur, pen and text on a captured picture. The editor is an ordinary local
//! page with the picture embedded in it; "Save" downloads the edited PNG like any other download.

use crate::app::App;

const TEMPLATE: &str = r##"<!doctype html><html><head><meta charset="utf-8"><title>Edit screenshot</title>
<style>
body{margin:0;background:#1f2430;font-family:"Segoe UI",system-ui,sans-serif;color:#e8ebf2}
#bar{position:sticky;top:0;z-index:5;display:flex;flex-wrap:wrap;gap:6px;align-items:center;padding:8px 12px;background:#2a3040;border-bottom:1px solid #3a4258}
button,input[type=color]{border:1px solid #4a5470;background:#343c52;color:#e8ebf2;border-radius:8px;padding:6px 12px;font:600 13px inherit;cursor:pointer;min-height:32px}
button:hover{background:#44506e}button.on{background:#10b981;border-color:#10b981;color:#fff}button.save{background:#10b981;border-color:#10b981;margin-left:auto}
input[type=color]{padding:0;width:36px;height:32px}label{font-size:12px;opacity:.8;display:flex;align-items:center;gap:6px}
#wrap{padding:14px;text-align:center}canvas{max-width:100%;height:auto;box-shadow:0 6px 30px rgba(0,0,0,.5);cursor:crosshair;background:#fff;touch-action:none}
#msg{font-size:12px;opacity:.8;margin-left:8px}
</style></head><body>
<div id="bar">
<button data-t="pen" class="on">Pen</button><button data-t="arrow">Arrow</button><button data-t="rect">Box</button>
<button data-t="hl">Highlight</button><button data-t="blur">Blur</button><button data-t="text">Text</button>
<input type="color" id="col" value="#ef4444" title="Colour">
<label>Size <input type="range" id="size" min="2" max="24" value="5"></label>
<button id="undo">Undo</button><button id="clear">Clear</button><span id="msg"></span>
<button class="save" id="save">Save PNG</button>
</div>
<div id="wrap"><canvas id="c"></canvas></div>
<script>
var img=new Image(),cv=document.getElementById('c'),ctx=cv.getContext('2d'),ops=[],cur=null,tool='pen';
img.onload=function(){cv.width=img.naturalWidth;cv.height=img.naturalHeight;render()};
img.src="data:image/png;base64,__IMG__";
function msg(t){var m=document.getElementById('msg');m.textContent=t;setTimeout(function(){m.textContent=''},3000)}
document.querySelectorAll('[data-t]').forEach(function(b){b.onclick=function(){tool=b.dataset.t;document.querySelectorAll('[data-t]').forEach(function(x){x.classList.toggle('on',x===b)})}});
function pos(e){var r=cv.getBoundingClientRect();return [(e.clientX-r.left)*cv.width/r.width,(e.clientY-r.top)*cv.height/r.height]}
function drawArrow(o){var a=o.p[0],b=o.p[o.p.length-1],w=o.size;ctx.strokeStyle=o.color;ctx.fillStyle=o.color;ctx.lineWidth=w;ctx.lineCap='round';
  ctx.beginPath();ctx.moveTo(a[0],a[1]);ctx.lineTo(b[0],b[1]);ctx.stroke();var ang=Math.atan2(b[1]-a[1],b[0]-a[0]),h=w*4+8;
  ctx.beginPath();ctx.moveTo(b[0],b[1]);ctx.lineTo(b[0]-h*Math.cos(ang-0.45),b[1]-h*Math.sin(ang-0.45));ctx.lineTo(b[0]-h*Math.cos(ang+0.45),b[1]-h*Math.sin(ang+0.45));ctx.closePath();ctx.fill()}
function box(o){var a=o.p[0],b=o.p[o.p.length-1];return [Math.min(a[0],b[0]),Math.min(a[1],b[1]),Math.abs(a[0]-b[0]),Math.abs(a[1]-b[1])]}
function drawOp(o){
  if(o.t==='pen'){ctx.strokeStyle=o.color;ctx.lineWidth=o.size;ctx.lineCap='round';ctx.lineJoin='round';ctx.beginPath();o.p.forEach(function(q,i){i?ctx.lineTo(q[0],q[1]):ctx.moveTo(q[0],q[1])});if(o.p.length===1)ctx.lineTo(o.p[0][0]+0.1,o.p[0][1]);ctx.stroke()}
  else if(o.t==='arrow'){drawArrow(o)}
  else if(o.t==='rect'){var r=box(o);ctx.strokeStyle=o.color;ctx.lineWidth=o.size;ctx.strokeRect(r[0],r[1],r[2],r[3])}
  else if(o.t==='hl'){var r=box(o);ctx.save();ctx.globalAlpha=0.38;ctx.fillStyle=o.color==='#ef4444'?'#facc15':o.color;ctx.fillRect(r[0],r[1],r[2],r[3]);ctx.restore()}
  else if(o.t==='blur'){var r=box(o),x=Math.max(0,Math.floor(r[0])),y=Math.max(0,Math.floor(r[1])),w=Math.min(cv.width-x,Math.ceil(r[2])),h=Math.min(cv.height-y,Math.ceil(r[3]));
    if(w>2&&h>2){var px=Math.max(8,o.size*2),t=document.createElement('canvas');t.width=Math.max(1,Math.round(w/px));t.height=Math.max(1,Math.round(h/px));
      var tc=t.getContext('2d');tc.imageSmoothingEnabled=true;tc.drawImage(cv,x,y,w,h,0,0,t.width,t.height);ctx.save();ctx.imageSmoothingEnabled=false;ctx.drawImage(t,0,0,t.width,t.height,x,y,w,h);ctx.restore()}}
  else if(o.t==='text'){ctx.fillStyle=o.color;ctx.font='700 '+(o.size*4+12)+'px Segoe UI,sans-serif';ctx.textBaseline='top';ctx.fillText(o.text,o.p[0][0],o.p[0][1])}
}
function render(){ctx.drawImage(img,0,0);ops.forEach(drawOp);if(cur)drawOp(cur)}
cv.addEventListener('pointerdown',function(e){var p=pos(e);var col=document.getElementById('col').value,size=+document.getElementById('size').value;
  if(tool==='text'){var t=prompt('Text');if(t){ops.push({t:'text',p:[p],color:col,size:size,text:t.slice(0,200)});render()}return}
  cur={t:tool,p:[p],color:col,size:size};cv.setPointerCapture(e.pointerId);render()});
cv.addEventListener('pointermove',function(e){if(!cur)return;var p=pos(e);if(cur.t==='pen')cur.p.push(p);else cur.p[1]=p;render()});
cv.addEventListener('pointerup',function(e){if(!cur)return;ops.push(cur);cur=null;render()});
document.getElementById('undo').onclick=function(){ops.pop();render()};
document.getElementById('clear').onclick=function(){ops=[];render()};
document.addEventListener('keydown',function(e){if((e.ctrlKey||e.metaKey)&&e.key==='z'){ops.pop();render()}});
document.getElementById('save').onclick=function(){render();cv.toBlob(function(b){var a=document.createElement('a');a.href=URL.createObjectURL(b);a.download='axomai-edited-'+Date.now()+'.png';document.body.appendChild(a);a.click();a.remove();msg('Saved to your Downloads folder')},'image/png')};
</script></body></html>"##;

pub fn editor_html(png: &[u8]) -> String {
    TEMPLATE.replace("__IMG__", &crate::sys::base64_encode(png))
}

/// `file:///` address of a local path.
pub fn file_url(path: &std::path::Path) -> String {
    let p = path.to_string_lossy().replace('\\', "/").replace(' ', "%20");
    format!("file:///{}", p.trim_start_matches('/'))
}

/// True for a PNG that lives in the screenshots folder.
pub fn is_capture(path: &std::path::Path, dir: &std::path::Path) -> bool {
    path.extension().map_or(false, |e| e.eq_ignore_ascii_case("png")) && path.parent().map_or(false, |p| p == dir)
}

impl App {
    /// "Edit" on the toast after a capture: open the picture in the editor in a new tab.
    pub fn open_screenshot_editor(&mut self) {
        let dir = crate::sys::screenshots_dir();
        let Some(path) = self.core.last_capture.clone().map(std::path::PathBuf::from).filter(|p| is_capture(p, &dir) && p.is_file()) else { return };
        let Ok(bytes) = std::fs::read(&path) else { return };
        if bytes.len() > 40_000_000 {
            return;
        }
        let out_dir = std::env::temp_dir().join("axomai-editor");
        let _ = std::fs::create_dir_all(&out_dir);
        let out = out_dir.join("edit.html");
        if std::fs::write(&out, editor_html(&bytes)).is_err() {
            return;
        }
        let at = self.tabs.len();
        self.open_tab_at(&file_url(&out), at, true);
        self.redraw = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn editor_embeds_the_picture_once() {
        let html = editor_html(&[137, 80, 78, 71]);
        assert!(html.contains("data:image/png;base64,iVBORw=="));
        assert!(!html.contains("__IMG__"));
        assert_eq!(html.matches('{').count(), html.matches('}').count());
    }

    #[test]
    fn file_urls_are_well_formed() {
        assert_eq!(file_url(Path::new("C:\\Users\\a b\\x.html")), "file:///C:/Users/a%20b/x.html");
    }

    #[test]
    fn only_pngs_from_the_screenshot_folder_are_editable() {
        let dir = Path::new("C:\\Pics\\Axomai Screenshots");
        assert!(is_capture(&dir.join("a.png"), dir));
        assert!(!is_capture(&dir.join("a.txt"), dir));
        assert!(!is_capture(Path::new("C:\\Windows\\a.png"), dir));
        assert!(!is_capture(&dir.join("sub").join("a.png"), dir));
    }
}
