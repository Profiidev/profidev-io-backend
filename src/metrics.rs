use std::collections::HashMap;

use chrono::{TimeZone, Utc};
use serde::{Deserialize, Serialize};
use surf::Client;
use tide::{Request, Response};

use crate::permissions::{has_permissions, Permissions};

#[derive(Deserialize, Debug)]
struct MetricsReq {
  start: i64,
  end: i64,
  step: i32,
}

#[derive(Deserialize, Debug)]
struct Metrics {
  data: MetricsData
}

#[derive(Deserialize, Debug)]
struct MetricsData {
  result: Vec<Core>,
}

#[derive(Deserialize, Debug)]
struct Core {
  metric: Key,
  values: Vec<Data>,
}

#[derive(Deserialize, Debug)]
struct Key {
  cpu: String,
}

#[derive(Deserialize, Debug)]
struct Data {
  time: i64,
  value: String,
}

#[derive(Serialize, Debug)]
struct MetricsRes {
  cores: HashMap<String, Vec<(i64, f32)>>,
}

pub(crate) async fn metrics(mut req: Request<()>) -> tide::Result {
    if !has_permissions(&req, Permissions::Metrics as i32) {
        return Ok(tide::Response::new(403));
    }

    let MetricsReq { start, end, step } = req.body_json().await?;
    if start > end || step < 1 {
      return Ok(tide::Response::new(400));
    }

    let metrics = get_metrics(start, end, step).await?;
    println!("{:?}", metrics.data.result.len());
    let mut cores: HashMap<String, Vec<(i64, f32)>> = HashMap::new();
    for core in metrics.data.result {
      cores.insert(core.metric.cpu, core.values.iter().map(|d| (d.time, d.value.parse::<f32>().unwrap())).collect());
    }
    
    let mut total: Vec<(i64, f32)> = Vec::new();
    for (_core, values) in cores.iter() {
      for (i, value) in values.iter().enumerate() {
        if i >= total.len() {
          total.push((value.0, value.1));
        } else {
          total[i].1 += value.1;
        }
      }
    }
    cores.insert("total".to_string(), total);
    let res_body = MetricsRes { cores };

    let mut res = Response::new(200);
    res.set_body(tide::Body::from_json(&res_body)?);
    Ok(res)
}

async fn get_metrics(start: i64, end: i64, step: i32) -> surf::Result<Metrics> {
  let client = Client::new();
  let start = Utc.timestamp_opt(start, 0).single().unwrap_or_default().format("%Y-%m-%dT%H:%M:%SZ").to_string();
  let end = Utc.timestamp_opt(end, 0).single().unwrap_or_default().format("%Y-%m-%dT%H:%M:%SZ").to_string();
  
  let url = format!("http://{}:9090/api/v1/query_range?query=sum by (cpu) (rate(node_cpu_seconds_total{{job=\"node\", mode!=\"idle\"}}[30s])) * 100&start={}&end={}&step={}m", *crate::METRICS_HOST, start, end, step);
  Ok(client.get(url).await?.body_json().await?)
}