function pipeline() { let t=0; for(let x=1;x<=100000;x++) { if(x%2) { let m=x*3; if(m%7===0) t+=m } } return t }
console.log(pipeline())
