function loop(n) { let t=0; for(let i=0;i<n;i++) for(let j=0;j<n;j++) t+=i*j; return t }
console.log(loop(200))
