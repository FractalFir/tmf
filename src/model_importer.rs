use crate::{FloatType, IndexType, TMFMesh, Vector2, Vector3};
pub(crate) struct ModelImporter {
    vertices: Vec<Vector3>,
    normals: Vec<Vector3>,
    uvs: Vec<Vector2>,
    vertex_triangles: Vec<IndexType>,
    normal_triangles: Vec<IndexType>,
    uv_triangles: Vec<IndexType>,
    name: String,
}
impl ModelImporter {
    pub(crate) fn new() -> Self {
        Self {
            vertices: Vec::new(),
            normals: Vec::new(),
            uvs: Vec::new(),
            vertex_triangles: Vec::new(),
            normal_triangles: Vec::new(),
            uv_triangles: Vec::new(),
            name: "".to_owned(),
        }
    }
    pub(crate) fn next_mesh(&mut self,mut name:String)->Option<(TMFMesh,String)>{
        if self.vertex_triangles.is_empty(){
            return None;
        }
        let mut mesh = TMFMesh::empty();
        let (mut vertices, mut vertex_triangles) = (self.vertices.clone(),self.vertex_triangles.clone());
        let (mut normals, mut normal_triangles) = (self.normals.clone(),self.normal_triangles.clone());
        let (mut uvs, mut uv_triangles) = (self.uvs.clone(),self.uv_triangles.clone());

        crate::utilis::fast_prune(&mut vertices,&mut vertex_triangles);
        crate::utilis::fast_prune(&mut normals,&mut normal_triangles);
        crate::utilis::fast_prune(&mut uvs,&mut uv_triangles);
        
        mesh.set_vertices(vertices);
        mesh.set_normals(normals);
        mesh.set_uvs(uvs);
        
        mesh.set_vertex_triangles(vertex_triangles);
        mesh.set_normal_triangles(normal_triangles);
        mesh.set_uv_triangles(uv_triangles);
        
        self.vertex_triangles.clear();
        self.normal_triangles.clear();
        self.uv_triangles.clear();
        
        std::mem::swap(&mut self.name,&mut name);
        
        Some((mesh,name))
    }
    pub(crate) fn finish(self)->(TMFMesh,String){
        let mut mesh = TMFMesh::empty();
        
        let (mut vertices, mut vertex_triangles) = (self.vertices.clone(),self.vertex_triangles.clone());
        let (mut normals, mut normal_triangles) = (self.normals.clone(),self.normal_triangles.clone());
        let (mut uvs, mut uv_triangles) = (self.uvs.clone(),self.uv_triangles.clone());
         
        crate::utilis::fast_prune(&mut vertices,&mut vertex_triangles);
        crate::utilis::fast_prune(&mut normals,&mut normal_triangles);
        crate::utilis::fast_prune(&mut uvs,&mut uv_triangles);
        
        mesh.set_vertices(vertices);
        mesh.set_normals(normals);
        mesh.set_uvs(uvs);
        
        mesh.set_vertex_triangles(vertex_triangles);
        mesh.set_normal_triangles(normal_triangles);
        mesh.set_uv_triangles(uv_triangles);
        
        (mesh,self.name)
    } 
    pub(crate) fn push_vertex(&mut self,vertex:Vector3){
        self.vertices.push(vertex);
    }
    pub(crate) fn push_normal(&mut self,normal:Vector3){
        self.normals.push(normal);
    }
    pub(crate) fn push_uv(&mut self,uv:Vector2){
        self.uvs.push(uv);
    }
    //Pushes raw index into vertex triangles. Intended to be used 
    pub(crate) fn push_vertex_index(&mut self,index:IndexType){
        self.vertex_triangles.push(index);
    }
    pub(crate) fn push_normal_index(&mut self,index:IndexType){
        self.normal_triangles.push(index);
    }
    pub(crate) fn push_uv_index(&mut self,index:IndexType){
        self.uv_triangles.push(index);
    }
}
