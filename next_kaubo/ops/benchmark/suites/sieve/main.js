function sieve(n) { let c=0; for(let p=2;p<=n;p++) { let ip=true; for(let d=2;d*d<=p;d++) { if(p%d===0) { ip=false; break } } if(ip) c++ } return c }
console.log(sieve(100000))
